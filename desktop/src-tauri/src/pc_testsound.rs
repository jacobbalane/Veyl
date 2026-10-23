//! Only an explicit PC sound check opens this render stream. Filtered check audio stays in RAM.
use crate::{
    pc_devices::AudioDevice,
    source::{AudioTap, PhoneSource},
};
use serde::Serialize;
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
use veyl_link::audio::AudioBlock;
#[cfg(windows)]
mod native;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckView {
    pub state: String,
    pub result: Option<CheckResult>,
    pub error: Option<String>,
}
impl Default for CheckView {
    fn default() -> Self {
        Self {
            state: "idle".into(),
            result: None,
            error: None,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub good: bool,
    pub mean_coherence: f32,
    pub coverage: f32,
    pub bandwidth_hz: f32,
    pub lag_ms: f32,
}
type Render =
    Arc<dyn Fn(AudioDevice, Arc<AtomicBool>, oneshot::Sender<()>) -> io::Result<()> + Send + Sync>;
pub struct Check {
    stop: Arc<AtomicBool>,
    pub result: oneshot::Receiver<Result<CheckResult, &'static str>>,
    finished: Option<oneshot::Receiver<()>>,
    task: Option<tauri::async_runtime::JoinHandle<()>>,
}
impl Check {
    pub(crate) fn start(
        device: AudioDevice,
        source: PhoneSource,
        generation: u64,
    ) -> io::Result<Self> {
        #[cfg(windows)]
        {
            Self::with_renderer(
                device,
                source,
                generation,
                Arc::new(native::play),
                Duration::from_secs(6),
            )
        }
        #[cfg(not(windows))]
        {
            let _ = (device, source, generation);
            Err(io::Error::other("pc_audio_error"))
        }
    }
    fn with_renderer(
        device: AudioDevice,
        source: PhoneSource,
        generation: u64,
        render: Render,
        duration: Duration,
    ) -> io::Result<Self> {
        let mut tap = source.tap(generation)?;
        let stop = Arc::new(AtomicBool::new(false));
        let render_stop = stop.clone();
        let (started, start) = oneshot::channel();
        let (finished, end) = oneshot::channel();
        let (render_done, rendered) = oneshot::channel();
        std::thread::Builder::new()
            .name("Veyl-test-sound".into())
            .spawn(move || {
                let result = render(device, render_stop, started).map_err(|_| "pc_audio_error");
                let _ = render_done.send(result);
                let _ = finished.send(());
            })?;
        let (done, result) = oneshot::channel();
        let task_stop = stop.clone();
        let task = tauri::async_runtime::spawn(async move {
            let result = collect(&mut tap, start, rendered, duration).await;
            task_stop.store(true, Ordering::Release);
            let _ = done.send(result);
        });
        Ok(Self {
            stop,
            result,
            finished: Some(end),
            task: Some(task),
        })
    }
    pub async fn shutdown(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(task) = self.task.take() {
            task.abort();
            let _ = task.await;
        }
        if let Some(done) = self.finished.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), done).await;
        }
    }
}
impl Drop for Check {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
struct Samples {
    channels: Option<u8>,
    expected: Option<u64>,
    samples: Vec<Vec<f32>>,
}
impl Samples {
    fn new() -> Self {
        Self {
            channels: None,
            expected: None,
            samples: vec![],
        }
    }
    fn push(&mut self, block: AudioBlock) -> Result<(), &'static str> {
        if block.sample_rate != 48000
            || !(1..=2).contains(&block.channels)
            || block.frames != 480
            || block.samples.len() != 480 * block.channels as usize
            || self.expected.is_some() && block.flags & 1 != 0
            || self
                .channels
                .is_some_and(|channels| channels != block.channels)
            || self
                .expected
                .is_some_and(|frame| frame != block.first_frame)
        {
            return Err("check_interrupted");
        }
        if self.channels.is_none() {
            self.channels = Some(block.channels);
            self.samples = (0..block.channels)
                .map(|_| Vec::with_capacity(48000 * 7))
                .collect();
        }
        self.expected = block.first_frame.checked_add(block.frames as u64);
        if self.samples[0].len() + 480 > 48000 * 7 {
            return Err("check_interrupted");
        }
        for frame in block.samples.chunks_exact(block.channels as usize) {
            for (channel, &sample) in frame.iter().enumerate() {
                self.samples[channel].push(sample as f32 / 32768.0);
            }
        }
        Ok(())
    }
}
async fn collect(
    tap: &mut AudioTap,
    mut started: oneshot::Receiver<()>,
    rendered: oneshot::Receiver<Result<(), &'static str>>,
    duration: Duration,
) -> Result<CheckResult, &'static str> {
    // Drain while the generator prepares, so preparation cannot overflow the bounded queue.
    let prepare = tokio::time::sleep(Duration::from_secs(20));
    tokio::pin!(prepare);
    loop {
        tokio::select! { biased; result = &mut started => { result.map_err(|_| "pc_audio_error")?; break; }, _ = &mut prepare => return Err("pc_audio_error"), block = tap.receiver.recv() => { if block.is_none() { return Err("check_interrupted"); } } }
    }
    let mut input = [Samples::new(), Samples::new()];
    let deadline = tokio::time::sleep(duration);
    tokio::pin!(deadline);
    loop {
        tokio::select! { _ = &mut deadline => break, block = tap.receiver.recv() => {
            let block = block.ok_or("check_interrupted")?; let stream = block.stream as usize;
            if stream > 1 { return Err("check_interrupted"); } input[stream].push(block)?;
        } }
    }
    if tap.overflow.load(Ordering::Acquire) {
        return Err("check_interrupted");
    }
    tokio::time::timeout(Duration::from_secs(2), rendered)
        .await
        .map_err(|_| "pc_audio_error")?
        .map_err(|_| "pc_audio_error")??;
    if input
        .iter()
        .any(|stream| stream.samples.is_empty() || stream.samples[0].len() < 48000)
    {
        return Err("check_interrupted");
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut best: Option<veyl_dsp::sound_check::Result> = None;
        for reference in &input[0].samples {
            for mic in &input[1].samples {
                let result = veyl_dsp::sound_check::analyze(reference, mic, 48000)
                    .map_err(|_| "check_interrupted")?;
                if best.as_ref().is_none_or(|best| result.score > best.score) {
                    best = Some(result);
                }
            }
        }
        let best = best.unwrap_or_default();
        Ok(CheckResult {
            good: best.mean_coherence >= 0.2 && best.effective_bandwidth_hz >= 2000.0,
            mean_coherence: best.mean_coherence,
            coverage: best.coverage,
            bandwidth_hz: best.effective_bandwidth_hz,
            lag_ms: best.lag_frames as f32 / 48.0,
        })
    })
    .await
    .map_err(|_| "check_interrupted")?
}
