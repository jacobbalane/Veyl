//! One owner switches sound sources and follows endpoint changes without opening a network in PC mode.
use crate::{
    connection::ConnectionManager,
    pc_devices::{AudioDevice, AudioDevices, DeviceMonitor},
    pc_source::{CaptureDetails, CaptureFactory, CaptureGroup},
    settings::Settings,
    source::PhoneSource,
};
use serde::Serialize;
use std::{
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceView {
    pub mode: String,
    pub choice_done: bool,
    pub state: String,
    pub speaker_id: Option<String>,
    pub microphone_id: Option<String>,
    pub speaker: Option<AudioDevice>,
    pub microphone: Option<AudioDevice>,
    pub reference: Option<CaptureDetails>,
    pub capture: Option<CaptureDetails>,
    pub devices: AudioDevices,
    pub tip: Option<String>,
    pub error: Option<String>,
    pub revision: u64,
}
impl Default for SourceView {
    fn default() -> Self {
        Self {
            mode: "phone".into(),
            choice_done: false,
            state: "idle".into(),
            speaker_id: None,
            microphone_id: None,
            speaker: None,
            microphone: None,
            reference: None,
            capture: None,
            devices: AudioDevices::default(),
            tip: None,
            error: None,
            revision: 0,
        }
    }
}
enum Change {
    Mode(String),
    Devices(Option<String>, Option<String>),
}
enum Command {
    Change(Change, oneshot::Sender<Result<(), String>>),
    Shutdown(oneshot::Sender<()>),
}
type Opening = Pin<Box<dyn Future<Output = io::Result<CaptureGroup>> + Send>>;
type Observer = Arc<dyn Fn(SourceView) + Send + Sync>;
#[cfg(test)]
mod tests;
#[derive(Clone)]
pub struct Sources {
    commands: mpsc::Sender<Command>,
    view: Arc<Mutex<SourceView>>,
    monitor: DeviceMonitor,
}
impl Sources {
    pub(crate) fn start(
        settings: Arc<Mutex<Settings>>,
        directory: &Path,
        connection: ConnectionManager,
        monitor: DeviceMonitor,
        observer: Observer,
    ) -> Self {
        let saved = settings.lock().unwrap().clone();
        let view = Arc::new(Mutex::new(SourceView {
            mode: saved.source,
            choice_done: saved.source_choice_done,
            speaker_id: saved.pc_speaker_id,
            microphone_id: saved.pc_mic_id,
            devices: monitor.snapshot(),
            ..Default::default()
        }));
        let (commands, receiver) = mpsc::channel(16);
        let actor = Actor {
            settings,
            path: directory.join("settings.json"),
            connection: connection.clone(),
            source: connection.source(),
            view: view.clone(),
            observer,
            current: None,
            opening: None,
            generation: 0,
            retry: None,
            factory: None,
        };
        let devices = monitor.subscribe();
        tauri::async_runtime::spawn(actor.run(receiver, devices));
        Self {
            commands,
            view,
            monitor,
        }
    }
    pub fn snapshot(&self) -> SourceView {
        self.view.lock().unwrap().clone()
    }
    async fn change(&self, change: Change) -> Result<(), String> {
        let (done, wait) = oneshot::channel();
        self.commands
            .try_send(Command::Change(change, done))
            .map_err(|_| "source_busy")?;
        wait.await.map_err(|_| "source_stopped")?
    }
    pub async fn set_source(&self, mode: String) -> Result<(), String> {
        self.change(Change::Mode(mode)).await
    }
    pub async fn set_devices(
        &self,
        speaker: Option<String>,
        mic: Option<String>,
    ) -> Result<(), String> {
        self.change(Change::Devices(speaker, mic)).await
    }
    pub async fn shutdown(&self) {
        let (done, wait) = oneshot::channel();
        if self.commands.send(Command::Shutdown(done)).await.is_ok() {
            let _ = wait.await;
        }
        self.monitor.shutdown().await;
    }
}
struct Actor {
    settings: Arc<Mutex<Settings>>,
    path: PathBuf,
    connection: ConnectionManager,
    source: PhoneSource,
    view: Arc<Mutex<SourceView>>,
    observer: Observer,
    current: Option<CaptureGroup>,
    opening: Option<Opening>,
    generation: u64,
    retry: Option<Instant>,
    factory: Option<CaptureFactory>,
}
impl Actor {
    fn snapshot(&self) -> SourceView {
        self.view.lock().unwrap().clone()
    }
    fn publish(&self) {
        let next = {
            let mut view = self.view.lock().unwrap();
            view.revision += 1;
            view.clone()
        };
        (self.observer)(next);
    }
    async fn stop_pc(&mut self) {
        self.retry = None;
        self.opening.take();
        // Fence callbacks from a cancelled preparation before another source can configure the engine.
        if self.generation != 0 {
            let _ = self.source.disconnected(self.generation).await;
            self.source.next_generation();
            self.generation = 0;
        }
        if let Some(group) = self.current.take() {
            group.shutdown().await;
        }
        let mut view = self.view.lock().unwrap();
        view.state = "idle".into();
        view.reference = None;
        view.capture = None;
    }
    fn save(&mut self, change: Change) -> Result<(), String> {
        let mut saved = self.settings.lock().unwrap();
        let mut next = saved.clone();
        match change {
            Change::Mode(mode) => {
                if !matches!(mode.as_str(), "phone" | "pc") {
                    return Err("invalid_source".into());
                }
                if mode == "pc" && next.source != "pc" && next.range == "normal" {
                    next.range = "far".into();
                }
                next.source = mode;
                next.source_choice_done = true;
            }
            Change::Devices(speaker, mic) => {
                let devices = &self.view.lock().unwrap().devices;
                if speaker.as_ref().is_some_and(|id| {
                    Some(id) != next.pc_speaker_id.as_ref()
                        && !devices.speakers.iter().any(|device| device.id == *id)
                }) || mic.as_ref().is_some_and(|id| {
                    Some(id) != next.pc_mic_id.as_ref()
                        && !devices.microphones.iter().any(|device| device.id == *id)
                }) {
                    return Err("pc_device_missing".into());
                }
                next.pc_speaker_id = speaker;
                next.pc_mic_id = mic;
            }
        }
        next.save(&self.path).map_err(|_| "settings_save_failed")?;
        let mut view = self.view.lock().unwrap();
        view.mode = next.source.clone();
        view.choice_done = next.source_choice_done;
        view.speaker_id = next.pc_speaker_id.clone();
        view.microphone_id = next.pc_mic_id.clone();
        *saved = next;
        Ok(())
    }
    async fn reconcile(&mut self) -> Result<(), String> {
        self.stop_pc().await;
        let snapshot = self.snapshot();
        self.connection.set_active(snapshot.mode == "phone").await?;
        let speaker = snapshot
            .devices
            .speaker(snapshot.speaker_id.as_deref())
            .cloned();
        let microphone = snapshot
            .devices
            .microphone(snapshot.microphone_id.as_deref())
            .cloned();
        {
            let mut view = self.view.lock().unwrap();
            view.speaker = speaker.clone();
            view.microphone = microphone.clone();
            view.error = None;
            view.tip = None;
            if snapshot.mode == "pc" {
                view.tip = if speaker.is_none() {
                    Some("pc_no_speaker")
                } else if microphone.is_none() {
                    Some("pc_no_mic")
                } else if speaker.as_ref().is_some_and(AudioDevice::headphones) {
                    Some("pc_headphones")
                } else if speaker.as_ref().is_some_and(|device| device.bluetooth) {
                    Some("pc_bluetooth")
                } else {
                    Some("nothing_playing")
                }
                .map(str::to_owned);
            }
        }
        if snapshot.mode == "pc" {
            if let (Some(speaker), Some(microphone)) = (speaker, microphone) {
                self.generation = self.source.next_generation();
                let source = self.source.clone();
                let generation = self.generation;
                let factory = self.factory.clone();
                self.opening = Some(Box::pin(async move {
                    match factory {
                        Some(factory) => {
                            CaptureGroup::with_factory(
                                speaker, microphone, source, generation, factory,
                            )
                            .await
                        }
                        None => CaptureGroup::start(speaker, microphone, source, generation).await,
                    }
                }));
                self.view.lock().unwrap().state = "starting".into();
            } else {
                self.view.lock().unwrap().state = "unavailable".into();
            }
        }
        self.publish();
        Ok(())
    }
    async fn failed(&mut self, code: &str) {
        self.stop_pc().await;
        let code = match code {
            "device_invalidated" => "device_invalidated",
            "mic_silenced" => "mic_silenced",
            _ => "pc_audio_error",
        };
        {
            let mut view = self.view.lock().unwrap();
            view.state = "error".into();
            view.error = Some(code.into());
            view.tip = Some(code.into());
        }
        self.retry = Some(
            Instant::now()
                + Duration::from_millis(if code == "device_invalidated" {
                    100
                } else {
                    5000
                }),
        );
        self.publish();
    }
    async fn run(
        mut self,
        mut commands: mpsc::Receiver<Command>,
        mut devices: tokio::sync::watch::Receiver<AudioDevices>,
    ) {
        self.view.lock().unwrap().devices = devices.borrow_and_update().clone();
        let _ = self.reconcile().await;
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        let mut done = None;
        loop {
            tokio::select! {
                command = commands.recv() => match command {
                    Some(Command::Change(change, done)) => { let result = self.save(change); let result = match result { Ok(()) => self.reconcile().await, Err(error) => Err(error) }; let _ = done.send(result); }
                    Some(Command::Shutdown(ack)) => { done = Some(ack); break; }, None => break,
                },
                result = async { match self.opening.as_mut() { Some(opening) => opening.await, None => std::future::pending().await } } => {
                    self.opening.take();
                    match result {
                        Ok(group) => { { let mut view = self.view.lock().unwrap(); view.reference = group.reference.clone(); view.capture = group.microphone.clone(); view.state = "running".into(); } self.current = Some(group); self.publish(); },
                        Err(error) => self.failed(&error.to_string()).await,
                    }
                },
                code = async { match self.current.as_mut() { Some(group) => group.next_failure().await, None => std::future::pending().await } } => { self.failed(code).await; },
                changed = devices.changed() => {
                    if changed.is_err() { break; }
                    let next = devices.borrow_and_update().clone();
                    let old = self.snapshot();
                    let restart = old.mode == "pc" && (old.devices.speaker(old.speaker_id.as_deref()) != next.speaker(old.speaker_id.as_deref()) || old.devices.microphone(old.microphone_id.as_deref()) != next.microphone(old.microphone_id.as_deref()));
                    self.view.lock().unwrap().devices = next;
                    if restart { let _ = self.reconcile().await; } else { self.publish(); }
                },
                _ = tick.tick(), if self.retry.is_some() => { if self.retry.is_some_and(|at| Instant::now() >= at) { let _ = self.reconcile().await; } }
            }
        }
        self.stop_pc().await;
        if let Some(done) = done {
            let _ = done.send(());
        }
    }
}
