//! M3 measures transport levels only. Echo processing is added in M4/M5.
use crate::source::PhoneSource;
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use tokio::sync::mpsc;
use veyl_dsp::{BlockFlags, BlockIn, Stream};
use veyl_link::{
    audio::AudioBlock,
    messages::{AudioConfig, Capture},
};

pub(crate) enum EngineInput {
    Config(u64, AudioConfig),
    Status(u64, Capture),
    Audio(u64, AudioBlock, Option<f64>),
    Disconnected(u64),
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamLevels {
    pub blocks: u64,
    pub blocks_per_second: f64,
    pub pre_dbfs: f32,
    pub filtered_dbfs: f32,
    pub peak: f32,
    pub gaps: u64,
    pub latest_frame: Option<u64>,
}
impl Default for StreamLevels {
    fn default() -> Self {
        Self {
            blocks: 0,
            blocks_per_second: 0.0,
            pre_dbfs: -120.0,
            filtered_dbfs: -120.0,
            peak: 0.0,
            gaps: 0,
            latest_frame: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LevelsView {
    pub run_id: Option<u64>,
    pub reference: StreamLevels,
    pub microphone: StreamLevels,
    pub sample_rate: Option<u32>,
    pub engine_drops: u64,
    pub arrival_ms: Option<f64>,
    pub running: bool,
}
pub struct EngineHost {
    view: Arc<Mutex<LevelsView>>,
    dropped: Arc<AtomicU64>,
}
impl EngineHost {
    pub fn start() -> (Self, PhoneSource) {
        Self::with_recorder(None)
    }
    pub fn with_recorder(recorder: Option<crate::recorder::Recorder>) -> (Self, PhoneSource) {
        let (sender, mut receiver) = mpsc::channel(400);
        let view = Arc::new(Mutex::new(LevelsView::default()));
        let dropped = Arc::new(AtomicU64::new(0));
        let source = PhoneSource::with_recorder(sender, dropped.clone(), recorder);
        let target = view.clone();
        std::thread::Builder::new()
            .name("Veyl-engine".into())
            .spawn(move || {
                let mut generation = 0;
                let mut format: Option<AudioConfig> = None;
                let mut expected = [None, None];
                let mut window = [Instant::now(); 2];
                let mut counts = [0u64; 2];
                while let Some(message) = receiver.blocking_recv() {
                    let mut state = target.lock().unwrap_or_else(|error| error.into_inner());
                    match message {
                        EngineInput::Config(next, config) if next >= generation => {
                            generation = next;
                            format = Some(config);
                            *state = LevelsView {
                                run_id: format.as_ref().map(|config| config.run_id),
                                sample_rate: format.as_ref().map(|config| config.sample_rate),
                                ..Default::default()
                            };
                            expected = [None, None];
                            window = [Instant::now(); 2];
                            counts = [0; 2];
                        }
                        EngineInput::Status(current, status) if current == generation => {
                            state.running = status.state == "running";
                            if !state.running {
                                state.reference.blocks_per_second = 0.0;
                                state.microphone.blocks_per_second = 0.0;
                            }
                        }
                        EngineInput::Disconnected(current) if current == generation => {
                            state.running = false;
                            state.reference.blocks_per_second = 0.0;
                            state.microphone.blocks_per_second = 0.0;
                        }
                        EngineInput::Audio(current, block, arrival)
                            if current == generation && state.running =>
                        {
                            let Some(config) = &format else { continue };
                            let index = block.stream as usize;
                            if index > 1
                                || block.sample_rate != config.sample_rate
                                || block.frames != config.frames_per_block
                                || block.channels
                                    != if index == 0 {
                                        config.ref_channels
                                    } else {
                                        config.mic_channels
                                    }
                            {
                                continue;
                            }
                            let input = input(&block);
                            let Some(end) = input.first_frame.checked_add(block.frames as u64)
                            else {
                                continue;
                            };
                            let level = if input.stream == Stream::Reference {
                                &mut state.reference
                            } else {
                                &mut state.microphone
                            };
                            let power = input
                                .samples
                                .iter()
                                .map(|&value| {
                                    let sample = value as f64 / 32768.0;
                                    sample * sample
                                })
                                .sum::<f64>()
                                / input.samples.len().max(1) as f64;
                            level.filtered_dbfs = (10.0 * power.max(1e-12).log10()) as f32;
                            level.pre_dbfs = input.pre_rms_dbfs.clamp(-120.0, 0.0);
                            level.peak = input.pre_peak.clamp(0.0, 1.0);
                            if let Some(frame) = expected[index] {
                                if input.first_frame > frame {
                                    level.gaps +=
                                        (input.first_frame - frame).div_ceil(block.frames as u64);
                                }
                            }
                            expected[index] = Some(expected[index].unwrap_or(0).max(end));
                            level.latest_frame = Some(end - 1);
                            level.blocks += 1;
                            counts[index] += 1;
                            let elapsed = window[index].elapsed().as_secs_f64();
                            if elapsed >= 1.0 {
                                level.blocks_per_second = counts[index] as f64 / elapsed;
                                counts[index] = 0;
                                window[index] = Instant::now();
                            }
                            state.arrival_ms =
                                arrival.filter(|value| value.is_finite() && *value >= 0.0);
                        }
                        _ => {}
                    }
                }
            })
            .expect("Engine worker could not start.");
        (Self { view, dropped }, source)
    }
    pub fn levels(&self) -> LevelsView {
        let mut view = self
            .view
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        view.engine_drops = self.dropped.load(Ordering::Relaxed);
        view
    }
}
fn input(block: &AudioBlock) -> BlockIn<'_> {
    BlockIn {
        stream: if block.stream == 0 {
            Stream::Reference
        } else {
            Stream::Microphone
        },
        first_frame: block.first_frame,
        capture_ns: if block.capture_ns == i64::MIN {
            None
        } else {
            Some(block.capture_ns)
        },
        flags: BlockFlags(block.flags),
        pre_rms_dbfs: block.pre_rms_dbfs,
        pre_peak: block.pre_peak,
        samples: &block.samples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SoundSource;
    use std::time::Duration;
    fn format(run: u64, rate: u32) -> AudioConfig {
        AudioConfig {
            run_id: run,
            source: "test_sound".into(),
            sample_rate: rate,
            frames_per_block: (rate / 100) as u16,
            ref_channels: 2,
            mic_channels: 1,
            mic_source: "unprocessed".into(),
            effects_off: vec![],
            hp_cutoff_hz: 8000,
            mics: vec![],
            sound_check: None,
        }
    }
    fn block(seq: u32, rate: u32, sample: i16) -> AudioBlock {
        AudioBlock {
            stream: 1,
            channels: 1,
            sample_rate: rate,
            seq,
            first_frame: seq as u64 * (rate / 100) as u64,
            capture_ns: i64::MIN,
            frames: (rate / 100) as u16,
            flags: 0,
            pre_rms_dbfs: -12.0,
            pre_peak: 0.5,
            samples: vec![sample; (rate / 100) as usize],
        }
    }
    fn wait_for(host: &EngineHost, predicate: impl Fn(&LevelsView) -> bool) -> LevelsView {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let view = host.levels();
            if predicate(&view) {
                return view;
            }
            assert!(Instant::now() < deadline, "Engine did not drain its queue.");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[tokio::test]
    async fn levels_count_gaps_and_reject_old_sessions_and_stopped_audio() {
        let (host, source) = EngineHost::start();
        source.configure(1, format(7, 48000)).await.unwrap();
        source
            .status(
                1,
                Capture {
                    state: "running".into(),
                    source: "test_sound".into(),
                    route: "speaker".into(),
                    reason: None,
                },
            )
            .await
            .unwrap();
        source.audio(1, block(0, 48000, 16384), Some(15.0)).unwrap();
        source.audio(0, block(1, 48000, 16384), None).unwrap();
        source.audio(1, block(2, 48000, 16384), Some(15.0)).unwrap();
        let first = wait_for(&host, |view| view.microphone.blocks == 2);
        assert_eq!(first.microphone.gaps, 1);
        assert_eq!(first.microphone.latest_frame, Some(1439));
        assert!((first.microphone.filtered_dbfs + 6.0206).abs() < 0.001);
        assert_eq!(first.arrival_ms, Some(15.0));
        assert_eq!(first.reference.blocks, 0);
        source.configure(2, format(8, 44100)).await.unwrap();
        source
            .status(
                2,
                Capture {
                    state: "running".into(),
                    source: "test_sound".into(),
                    route: "speaker".into(),
                    reason: None,
                },
            )
            .await
            .unwrap();
        source.audio(1, block(3, 48000, 16384), None).unwrap();
        source.audio(2, block(0, 44100, 0), None).unwrap();
        let second = wait_for(&host, |view| {
            view.run_id == Some(8) && view.microphone.blocks == 1
        });
        assert_eq!(second.sample_rate, Some(44100));
        assert_eq!(second.microphone.filtered_dbfs, -120.0);
        assert_eq!(second.microphone.gaps, 0);
        assert!(source.configure(1, format(7, 48000)).await.is_err());
        assert!(!source.audio(1, block(4, 48000, 1), None).unwrap());
        assert_eq!(host.levels().run_id, Some(8));
        source
            .status(
                2,
                Capture {
                    state: "idle".into(),
                    source: "test_sound".into(),
                    route: "speaker".into(),
                    reason: Some("user".into()),
                },
            )
            .await
            .unwrap();
        source.audio(2, block(1, 44100, 1), None).unwrap();
        wait_for(&host, |view| !view.running);
        source.disconnected(2).await.unwrap();
        assert_eq!(host.levels().microphone.blocks, 1);
    }
}
