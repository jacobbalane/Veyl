use crate::{
    align::{Aligner, LockChange},
    background::Background,
    config::EngineConfig,
    echo::{EchoEstimator, EchoFrame},
    meter::{Meter, MeterReading, SourceStatus},
    motion::{MotionFeatures, MotionInput, MotionTracker},
    poke::{PokeDetector, PokeEvent},
    ring::SampleRing,
    BlockFlags, BlockIn, RangePreset, Sensitivity, Stream, StreamFormat,
};
use serde::{Deserialize, Serialize};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineState {
    #[default]
    NoAudio,
    WaitingForSound,
    Aligning,
    Settling,
    Running,
}

#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Warning {
    Relocked,
    ModelJump,
    ImpulsiveNoise,
    Drift,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum EngineEvent {
    State(EngineState),
    Aligned {
        lag_frames: i64,
        lag_ms: f32,
        psr: f32,
    },
    Hand {
        present: bool,
        distance_cm: Option<f32>,
    },
    Features(MotionFeatures),
    Poke(PokeEvent),
    Meter(MeterReading),
    Warning(Warning),
}
#[derive(Clone, Copy, Debug)]
pub struct EngineSnapshot {
    pub state: EngineState,
    pub frame: Option<u64>,
    pub lag_frames: Option<i64>,
    pub drift_ppm: f64,
    pub direct_residual: Option<f64>,
    pub hand: MotionFeatures,
    pub motion: [f32; 60],
    pub echo: Option<EchoFrame>,
    pub meter: MeterReading,
    pub suspended: bool,
    pub paused: bool,
    pub processed_hops: u64,
    pub invalid_hops: u64,
}

/// Pure processing; capture, transport, recording and UI are owned by the caller.
pub struct Engine {
    config: EngineConfig,
    format: StreamFormat,
    reference: SampleRing,
    microphone: SampleRing,
    aligner: Aligner,
    echo: EchoEstimator,
    background: Background,
    motion: MotionTracker,
    detector: PokeDetector,
    meter: Meter,
    reference_window: Vec<f32>,
    microphone_window: Vec<f32>,
    state: EngineState,
    next_hop: Option<u64>,
    last_frame: Option<u64>,
    latest_echo: Option<EchoFrame>,
    reference_power: f32,
    running_frames: u64,
    suspended_until: u64,
    model_jump_until: u64,
    paused: bool,
    impulsive_noise: bool,
    drift_warning: bool,
    row: Cell<[f32; 60]>,
    processed_hops: u64,
    invalid_hops: u64,
    pending_hand_end: bool,
}
impl Engine {
    pub fn new(config: EngineConfig, format: StreamFormat) -> Result<Self, &'static str> {
        config.validate()?;
        format.validate()?;
        Ok(Self {
            reference: SampleRing::new(
                format.sample_rate,
                format.ref_channels,
                config.ring_seconds,
            )?,
            microphone: SampleRing::new(
                format.sample_rate,
                format.mic_channels,
                config.ring_seconds,
            )?,
            aligner: Aligner::new(config.clone(), format, config.pc_mode)?,
            echo: EchoEstimator::new(config.clone(), format)?,
            background: Background::new(config.clone(), format)?,
            motion: MotionTracker::new(config.clone(), format)?,
            detector: PokeDetector::new(config.clone(), format.sample_rate)?,
            meter: Meter::new(
                format.sample_rate,
                config.hop,
                config.pc_mode,
                config.temperature_c,
            ),
            reference_window: vec![0.0; config.window * format.ref_channels as usize],
            microphone_window: vec![0.0; config.window * format.mic_channels as usize],
            state: EngineState::NoAudio,
            next_hop: None,
            last_frame: None,
            latest_echo: None,
            reference_power: 0.0,
            running_frames: 0,
            suspended_until: 0,
            model_jump_until: 0,
            paused: false,
            impulsive_noise: false,
            drift_warning: false,
            row: Cell::new([0.0; 60]),
            processed_hops: 0,
            invalid_hops: 0,
            pending_hand_end: false,
            config,
            format,
        })
    }
    pub fn push(&mut self, block: BlockIn<'_>) -> Result<(), &'static str> {
        let channels = match block.stream {
            Stream::Reference => self.format.ref_channels,
            Stream::Microphone => self.format.mic_channels,
        } as usize;
        if block.samples.len() != self.format.sample_rate as usize / 100 * channels
            || block.flags.0 & !15 != 0
            || !block.pre_rms_dbfs.is_finite()
            || !block.pre_peak.is_finite()
            || !(0.0..=1.0).contains(&block.pre_peak)
        {
            return Err("Invalid engine audio block.");
        }
        let ring = match block.stream {
            Stream::Reference => &mut self.reference,
            Stream::Microphone => &mut self.microphone,
        };
        let accepted = ring.push(
            block.first_frame,
            block.samples,
            block.capture_ns,
            block.flags.0 & BlockFlags::GAP_BEFORE != 0,
        )?;
        if accepted.frames == block.samples.len() / channels {
            self.meter.observe_block(&block, channels);
        }
        if accepted.frames > 0 && block.stream == Stream::Reference {
            let power = block
                .samples
                .iter()
                .map(|sample| (*sample as f32 / 32768.0).powi(2))
                .sum::<f32>()
                / block.samples.len() as f32;
            let alpha = 1.0 - (-0.01_f32 / 0.1).exp();
            self.reference_power += alpha * (power - self.reference_power);
        }
        Ok(())
    }
    fn set_state(&mut self, state: EngineState, events: &mut Vec<EngineEvent>) {
        if self.state != state {
            self.state = state;
            events.push(EngineEvent::State(state));
        }
    }
    fn reset_motion(&mut self, short: bool, events: &mut Vec<EngineEvent>) {
        if self.motion.present() || self.pending_hand_end {
            events.push(EngineEvent::Hand {
                present: false,
                distance_cm: None,
            });
        }
        self.pending_hand_end = false;
        self.background.reset(short);
        self.motion.reset();
        self.detector.suspend();
        self.running_frames = 0;
        self.row.set([0.0; 60]);
    }
    pub fn set_sensitivity(&mut self, preset: Sensitivity) {
        self.pending_hand_end |= self.motion.present();
        self.motion.set_sensitivity(preset);
        self.detector.set_sensitivity(preset);
    }
    pub fn set_range(&mut self, preset: RangePreset) {
        self.pending_hand_end |= self.motion.present();
        self.motion.set_range(preset);
        self.detector.set_range(preset);
    }
    pub fn set_paused(&mut self, paused: bool) {
        if self.paused != paused {
            self.detector.suspend();
        }
        self.paused = paused;
    }
    pub fn set_source_status(&mut self, status: SourceStatus) -> Result<(), &'static str> {
        let changed = self.meter.status() != status;
        self.meter.set_status(status)?;
        if changed {
            self.detector.suspend();
        }
        Ok(())
    }
    pub fn reset(&mut self) {
        self.reference.clear();
        self.microphone.clear();
        self.aligner.reset();
        self.echo.reset();
        self.background.reset(false);
        self.motion.reset();
        self.detector.reset();
        self.meter = Meter::new(
            self.format.sample_rate,
            self.config.hop,
            self.config.pc_mode,
            self.config.temperature_c,
        );
        self.row.set([0.0; 60]);
        self.state = EngineState::NoAudio;
        self.next_hop = None;
        self.last_frame = None;
        self.latest_echo = None;
        self.reference_power = 0.0;
        self.running_frames = 0;
        self.suspended_until = 0;
        self.model_jump_until = 0;
        self.impulsive_noise = false;
        self.drift_warning = false;
        self.processed_hops = 0;
        self.invalid_hops = 0;
        self.pending_hand_end = false;
    }
    pub fn snapshot(&self) -> EngineSnapshot {
        EngineSnapshot {
            state: self.state,
            frame: self.last_frame,
            lag_frames: self.aligner.locked().then(|| self.aligner.lag()),
            drift_ppm: self.aligner.drift_ppm(),
            direct_residual: self.aligner.direct_residual(),
            hand: self.motion.features(),
            motion: self.row.replace([0.0; 60]),
            echo: self.latest_echo,
            meter: self.meter.reading(),
            suspended: self.paused
                || !self.meter.status().permits_detection(self.config.pc_mode)
                || self.meter.reading().score < 15
                || !self.meter.reference_active()
                || self.state != EngineState::Running
                || self
                    .last_frame
                    .is_none_or(|frame| frame < self.suspended_until),
            paused: self.paused,
            processed_hops: self.processed_hops,
            invalid_hops: self.invalid_hops,
        }
    }
    /// The caller can reuse an event Vec; this method never clears its existing entries.
    pub fn process(&mut self, events: &mut Vec<EngineEvent>) -> Result<(), &'static str> {
        self.process_hops(events)?;
        if let Some(reading) = self.meter.tick(
            self.microphone.end().unwrap_or(0),
            self.reference.end().is_some() && self.microphone.end().is_some(),
            self.impulsive_noise,
        ) {
            events.push(EngineEvent::Meter(reading));
        }
        Ok(())
    }
    fn process_hops(&mut self, events: &mut Vec<EngineEvent>) -> Result<(), &'static str> {
        if self.pending_hand_end {
            events.push(EngineEvent::Hand {
                present: false,
                distance_cm: None,
            });
            self.pending_hand_end = false;
        }
        let (Some(_), Some(mic_end)) = (self.reference.end(), self.microphone.end()) else {
            self.set_state(EngineState::NoAudio, events);
            return Ok(());
        };
        let minimum_power = 10.0_f32.powf(self.config.align_min_dbfs / 10.0);
        if self.reference_power < minimum_power {
            if matches!(self.state, EngineState::Running | EngineState::Settling) {
                self.echo.reset_averages();
                self.reset_motion(true, events);
            }
            self.set_state(EngineState::WaitingForSound, events);
            if self.aligner.locked() {
                self.next_hop = Some(mic_end - 1);
            }
            return Ok(());
        }
        if let Some(change) = self.aligner.try_align(&self.reference, &self.microphone) {
            let estimate = match change {
                LockChange::Locked(value) | LockChange::Relocked(value) => value,
            };
            let relocked = matches!(change, LockChange::Relocked(_));
            self.echo.reset();
            self.reset_motion(relocked, events);
            self.next_hop = Some(mic_end - 1);
            self.suspended_until = mic_end.saturating_add(
                (self.config.jump_suspend_ms * self.format.sample_rate as f32 / 1000.0) as u64,
            );
            if relocked {
                events.push(EngineEvent::Warning(Warning::Relocked));
            }
            events.push(EngineEvent::Aligned {
                lag_frames: estimate.lag_frames,
                lag_ms: estimate.fractional_lag as f32 * 1000.0 / self.format.sample_rate as f32,
                psr: estimate.psr,
            });
        }
        if !self.aligner.locked() {
            self.set_state(EngineState::Aligning, events);
            return Ok(());
        }
        while let Some(frame) = self.next_hop.filter(|frame| *frame < mic_end) {
            let Some((lag, _)) = self.aligner.preview_hop(frame) else {
                break;
            };
            let Some(mic_first) = frame
                .checked_add(1)
                .and_then(|end| end.checked_sub(self.config.window as u64))
            else {
                break;
            };
            let Ok(ref_first) = u64::try_from(mic_first as i128 - lag as i128) else {
                break;
            };
            if ref_first
                .checked_add(self.config.window as u64)
                .is_none_or(|end| end > self.reference.end().unwrap())
            {
                break;
            }
            let (_, fraction) = self
                .aligner
                .begin_hop(frame)
                .ok_or("Non-increasing engine hop.")?;
            let mut valid = true;
            for channel in 0..self.format.ref_channels as usize {
                valid &= self.reference.copy(
                    ref_first,
                    Some(channel),
                    &mut self.reference_window
                        [channel * self.config.window..(channel + 1) * self.config.window],
                );
            }
            for channel in 0..self.format.mic_channels as usize {
                valid &= self.microphone.copy(
                    mic_first,
                    Some(channel),
                    &mut self.microphone_window
                        [channel * self.config.window..(channel + 1) * self.config.window],
                );
            }
            self.last_frame = Some(frame);
            self.processed_hops += 1;
            self.next_hop = frame.checked_add(self.config.hop as u64);
            if !valid {
                self.invalid_hops += 1;
                self.suspended_until = frame.saturating_add(
                    (self.config.jump_suspend_ms * self.format.sample_rate as f32 / 1000.0) as u64,
                );
                let features = self.motion.process(MotionInput {
                    frame,
                    dynamic: self.background.dynamic(),
                    normalized: None,
                    direct_position: 0.0,
                    direct_energy: 0.0,
                    reference_weights: [1.0, 0.0],
                    reference_cross: Default::default(),
                    settling: !self.background.settled(),
                    held: true,
                })?;
                events.push(EngineEvent::Features(features));
                self.detector.suspend();
                continue;
            }
            let fast = self.echo.process(
                &self.reference_window,
                &self.microphone_window,
                fraction,
                self.background
                    .settled()
                    .then(|| self.background.transfer()),
                self.running_frames >= self.format.sample_rate as u64,
            )?;
            if !fast.impulse {
                self.aligner.observe_fast(fast.direct, fast.mode_changed);
            }
            if fast.mode_changed {
                let short = self.background.settled();
                self.reset_motion(short, events);
            }
            let background = self.background.process(
                self.echo.transfer(),
                self.echo.maps(),
                self.echo.weights(),
                self.motion.present(),
                fast.impulse || frame < self.model_jump_until,
            )?;
            if background.model_jump {
                events.push(EngineEvent::Warning(Warning::ModelJump));
                self.echo.reset_averages();
                self.suspended_until = frame.saturating_add(
                    (self.config.jump_suspend_ms * self.format.sample_rate as f32 / 1000.0) as u64,
                );
                self.model_jump_until = self.suspended_until;
            }
            let features = self.motion.process(MotionInput {
                frame,
                dynamic: self.background.dynamic(),
                normalized: Some(self.background.normalized_maps()),
                direct_position: background.direct.position,
                direct_energy: background.direct_energy,
                reference_weights: fast.reference_weights,
                reference_cross: fast.reference_cross,
                settling: background.settling,
                held: fast.impulse || background.model_jump || frame < self.model_jump_until,
            })?;
            if features.changed {
                events.push(EngineEvent::Hand {
                    present: features.present,
                    distance_cm: features.distance_cm,
                });
            }
            events.push(EngineEvent::Features(features));
            if let Some(poke) = self.detector.process(
                features,
                !background.settling
                    && !self.paused
                    && frame >= self.suspended_until
                    && self.meter.status().permits_detection(self.config.pc_mode)
                    && self.meter.reading().score >= 15
                    && self.meter.reference_active(),
            ) {
                events.push(EngineEvent::Poke(poke));
            }
            let mut row = self.row.get();
            for (maximum, value) in row.iter_mut().zip(self.motion.take_row()) {
                *maximum = maximum.max(value);
            }
            self.row.set(row);
            self.set_state(
                if background.settling {
                    EngineState::Settling
                } else {
                    EngineState::Running
                },
                events,
            );
            if !background.settling {
                self.running_frames += self.config.hop as u64;
            }
            let impulsive = fast.impulse_fraction > 0.3;
            if impulsive && !self.impulsive_noise {
                events.push(EngineEvent::Warning(Warning::ImpulsiveNoise));
            }
            self.impulsive_noise = impulsive;
            let drift = self.aligner.drift_warning();
            if drift && !self.drift_warning {
                events.push(EngineEvent::Warning(Warning::Drift));
            }
            self.drift_warning = drift;
            self.latest_echo = Some(fast);
            self.meter.observe_echo(frame, fast);
        }
        Ok(())
    }
}
