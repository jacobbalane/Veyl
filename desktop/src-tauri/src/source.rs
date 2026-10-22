//! Sources deliver the same filtered blocks and state to the engine's bounded queue.
use crate::{engine_host::EngineInput, recorder::Recorder};
use std::{
    future::Future,
    io,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{sync::mpsc, time::timeout};
use veyl_link::{
    audio::AudioBlock,
    messages::{AudioConfig, Capture},
};

pub trait SoundSource {
    fn configure(
        &self,
        generation: u64,
        format: AudioConfig,
    ) -> impl Future<Output = io::Result<()>> + Send;
    fn status(
        &self,
        generation: u64,
        status: Capture,
    ) -> impl Future<Output = io::Result<()>> + Send;
    fn audio(
        &self,
        generation: u64,
        block: AudioBlock,
        arrival_ms: Option<f64>,
    ) -> io::Result<bool>;
}
#[derive(Clone)]
pub struct PhoneSource {
    sender: mpsc::Sender<EngineInput>,
    dropped: Arc<AtomicU64>,
    gaps: Arc<[AtomicBool; 2]>,
    recorder: Option<Recorder>,
    generation: Arc<AtomicU64>,
}
impl PhoneSource {
    #[cfg(test)]
    pub(crate) fn new(sender: mpsc::Sender<EngineInput>, dropped: Arc<AtomicU64>) -> Self {
        Self::with_recorder(sender, dropped, None)
    }
    pub(crate) fn with_recorder(
        sender: mpsc::Sender<EngineInput>,
        dropped: Arc<AtomicU64>,
        recorder: Option<Recorder>,
    ) -> Self {
        Self {
            sender,
            dropped,
            gaps: Arc::new([AtomicBool::new(false), AtomicBool::new(false)]),
            recorder,
            generation: Arc::new(AtomicU64::new(0)),
        }
    }
    pub(crate) fn next_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::AcqRel) + 1
    }
    pub async fn disconnected(&self, generation: u64) -> io::Result<()> {
        if let Some(recorder) = &self.recorder {
            recorder.disconnected(generation);
        }
        self.control(EngineInput::Disconnected(generation)).await
    }
    pub(crate) fn disconnect_now(&self, generation: u64) {
        if let Some(recorder) = &self.recorder {
            recorder.disconnected(generation);
        }
        let _ = self.sender.try_send(EngineInput::Disconnected(generation));
    }
    pub(crate) fn phone_ready(&self, generation: u64, ready: &veyl_link::messages::Ready) {
        if generation < self.generation.load(Ordering::Acquire) {
            return;
        }
        if let (Some(recorder), veyl_link::messages::Ready::Phone { device, .. }) =
            (&self.recorder, ready)
        {
            recorder.source_info(serde_json::json!(device), serde_json::json!({"desktop": env!("CARGO_PKG_VERSION"), "phone": device.app_version}));
        }
    }
    pub(crate) fn source_info(
        &self,
        generation: u64,
        device: serde_json::Value,
        versions: serde_json::Value,
    ) {
        if generation < self.generation.load(Ordering::Acquire) {
            return;
        }
        if let Some(recorder) = &self.recorder {
            recorder.source_info(device, versions);
        }
    }
    async fn control(&self, message: EngineInput) -> io::Result<()> {
        timeout(Duration::from_secs(1), self.sender.send(message))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Engine queue stalled."))?
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Engine stopped."))
    }
}
impl SoundSource for PhoneSource {
    async fn configure(&self, generation: u64, format: AudioConfig) -> io::Result<()> {
        if generation < self.generation.fetch_max(generation, Ordering::AcqRel) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Sound source was replaced.",
            ));
        }
        if ![44100, 48000].contains(&format.sample_rate)
            || format.frames_per_block as u32 != format.sample_rate / 100
            || !(1..=2).contains(&format.ref_channels)
            || !(1..=4).contains(&format.mic_channels)
            || ![6000, 7000, 8000].contains(&format.hp_cutoff_hz)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported audio configuration.",
            ));
        }
        self.gaps
            .iter()
            .for_each(|flag| flag.store(false, Ordering::Release));
        self.control(EngineInput::Config(generation, format.clone()))
            .await?;
        if let Some(recorder) = &self.recorder {
            recorder.configure(generation, format);
        }
        Ok(())
    }
    async fn status(&self, generation: u64, status: Capture) -> io::Result<()> {
        if generation != self.generation.load(Ordering::Acquire) {
            return Ok(());
        }
        self.control(EngineInput::Status(generation, status.clone()))
            .await?;
        if let Some(recorder) = &self.recorder {
            recorder.status(generation, &status);
        }
        Ok(())
    }
    fn audio(
        &self,
        generation: u64,
        mut block: AudioBlock,
        arrival_ms: Option<f64>,
    ) -> io::Result<bool> {
        if generation != self.generation.load(Ordering::Acquire) {
            return Ok(false);
        }
        if let Some(recorder) = &self.recorder {
            recorder.audio(generation, &block);
        }
        let gap = self
            .gaps
            .get(block.stream as usize)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid stream."))?;
        if gap.swap(false, Ordering::AcqRel) {
            block.flags |= 1;
        }
        match self
            .sender
            .try_send(EngineInput::Audio(generation, block, arrival_ms))
        {
            Ok(()) => Ok(true),
            Err(mpsc::error::TrySendError::Full(_)) => {
                gap.store(true, Ordering::Release);
                self.dropped.fetch_add(1, Ordering::Relaxed);
                Ok(false)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "Engine stopped."))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn block(seq: u32) -> AudioBlock {
        AudioBlock {
            stream: 0,
            channels: 2,
            sample_rate: 48000,
            seq,
            first_frame: seq as u64 * 480,
            capture_ns: i64::MIN,
            frames: 480,
            flags: 0,
            pre_rms_dbfs: -20.0,
            pre_peak: 0.2,
            samples: vec![0; 960],
        }
    }
    #[test]
    fn full_engine_queue_drops_audio_without_waiting_and_marks_the_next_block() {
        let (tx, mut rx) = mpsc::channel(2);
        let dropped = Arc::new(AtomicU64::new(0));
        let source = PhoneSource::new(tx, dropped.clone());
        assert_eq!(source.next_generation(), 1);
        assert!(source.audio(1, block(0), None).unwrap());
        assert!(source.audio(1, block(1), None).unwrap());
        assert!(!source.audio(1, block(2), None).unwrap());
        assert!(!source.audio(1, block(3), None).unwrap());
        assert_eq!(dropped.load(Ordering::Relaxed), 2);
        rx.try_recv().unwrap();
        rx.try_recv().unwrap();
        assert!(source.audio(1, block(4), None).unwrap());
        let EngineInput::Audio(_, next, _) = rx.try_recv().unwrap() else {
            panic!("Expected audio.")
        };
        assert_eq!(next.seq, 4);
        assert_eq!(next.flags & 1, 1);
        drop(rx);
        assert!(source.audio(1, block(5), None).is_err());
    }
}
