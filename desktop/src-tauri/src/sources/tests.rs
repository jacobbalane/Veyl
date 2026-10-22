use super::*;
use crate::{
    connection::Command as LinkCommand,
    pc_source::{CaptureBackend, CaptureFormat, CapturePacket, Encoding},
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Fake {
    frame: u64,
    live: Arc<AtomicUsize>,
    started: Arc<AtomicUsize>,
}
impl Drop for Fake {
    fn drop(&mut self) {
        self.live.fetch_sub(1, Ordering::AcqRel);
    }
}
impl CaptureBackend for Fake {
    fn details(&self) -> CaptureDetails {
        CaptureDetails {
            format: CaptureFormat {
                sample_rate: 48000,
                channels: 1,
                encoding: Encoding::Signed16,
                frame_stride: 2,
            },
            raw_mode: false,
            event_driven: false,
            buffer_ms: 20.0,
            pro_audio_thread: false,
        }
    }
    fn start(&mut self) -> io::Result<()> {
        self.started.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
    fn wait(&mut self) -> io::Result<()> {
        std::thread::sleep(Duration::from_millis(3));
        Ok(())
    }
    fn poll(
        &mut self,
        sink: &mut dyn FnMut(CapturePacket<'_>) -> io::Result<()>,
    ) -> io::Result<()> {
        sink(CapturePacket {
            bytes: &[],
            frames: 480,
            device_position: Some(self.frame),
            capture_ns: None,
            discontinuity: false,
            silent: true,
        })?;
        self.frame += 480;
        Ok(())
    }
    fn stop(&mut self) {}
}
fn device(id: &str) -> AudioDevice {
    AudioDevice {
        id: id.into(),
        name: id.into(),
        form_factor: "speakers".into(),
        raw_supported: false,
        bluetooth: false,
        built_in: true,
    }
}
fn devices() -> AudioDevices {
    AudioDevices {
        speakers: vec![device("speaker-a"), device("speaker-b")],
        microphones: vec![device("mic")],
        default_speaker_id: Some("speaker-a".into()),
        default_microphone_id: Some("mic".into()),
        error: None,
        ..Default::default()
    }
}
fn setup(
    path: &Path,
    live: Arc<AtomicUsize>,
    started: Arc<AtomicUsize>,
    gate: Option<Arc<AtomicBool>>,
) -> (Actor, Arc<AtomicBool>) {
    let settings = Arc::new(Mutex::new(
        Settings::load_or_create(&path.join("settings.json")).unwrap(),
    ));
    let (connection, mut commands) = ConnectionManager::isolated();
    let network = Arc::new(AtomicBool::new(true));
    let active = network.clone();
    tokio::spawn(async move {
        while let Some(command) = commands.recv().await {
            match command {
                LinkCommand::SetActive(next, done) => {
                    active.store(next, Ordering::Release);
                    let _ = done.send(());
                }
                LinkCommand::Shutdown(done) => {
                    let _ = done.send(());
                    break;
                }
                _ => {}
            }
        }
    });
    let source = connection.source();
    let factory: CaptureFactory = Arc::new(move |_, stream| {
        if let Some(gate) = &gate {
            if stream == 1 {
                while !gate.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
        live.fetch_add(1, Ordering::AcqRel);
        Ok(Box::new(Fake {
            frame: 0,
            live: live.clone(),
            started: started.clone(),
        }))
    });
    (
        Actor {
            settings,
            path: path.join("settings.json"),
            connection,
            source,
            view: Arc::new(Mutex::new(SourceView {
                devices: devices(),
                ..Default::default()
            })),
            observer: Arc::new(|_| {}),
            current: None,
            opening: None,
            generation: 0,
            retry: None,
            factory: Some(factory),
        },
        network,
    )
}
async fn wait(view: &Arc<Mutex<SourceView>>, predicate: impl Fn(&SourceView) -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !predicate(&view.lock().unwrap()) {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap();
}
async fn change(commands: &mpsc::Sender<Command>, change: Change) -> Result<(), String> {
    let (done, wait) = oneshot::channel();
    commands.send(Command::Change(change, done)).await.unwrap();
    wait.await.unwrap()
}
#[tokio::test]
async fn default_changes_restart_selected_streams_and_missing_explicit_devices_are_never_substituted(
) {
    let folder = tempfile::tempdir().unwrap();
    let live = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(AtomicUsize::new(0));
    let (actor, network) = setup(folder.path(), live.clone(), started.clone(), None);
    let view = actor.view.clone();
    let (commands, rx) = mpsc::channel(16);
    let (devices, device_rx) = tokio::sync::watch::channel(devices());
    let task = tokio::spawn(actor.run(rx, device_rx));
    change(&commands, Change::Mode("pc".into())).await.unwrap();
    wait(&view, |v| v.state == "running").await;
    assert!(!network.load(Ordering::Acquire));
    assert_eq!(live.load(Ordering::Acquire), 2);
    devices.send_modify(|d| {
        d.default_speaker_id = Some("speaker-b".into());
        d.revision += 1;
    });
    wait(&view, |v| {
        v.state == "running" && v.speaker.as_ref().unwrap().id == "speaker-b"
    })
    .await;
    assert_eq!(started.load(Ordering::Acquire), 4);
    assert_eq!(live.load(Ordering::Acquire), 2);
    change(
        &commands,
        Change::Devices(Some("speaker-a".into()), Some("mic".into())),
    )
    .await
    .unwrap();
    wait(&view, |v| v.state == "running").await;
    devices.send_modify(|d| {
        d.speakers.retain(|d| d.id != "speaker-a");
        d.revision += 1;
    });
    wait(&view, |v| v.state == "unavailable").await;
    assert!(view.lock().unwrap().speaker.is_none());
    assert_eq!(live.load(Ordering::Acquire), 0);
    let saved = Settings::load_or_create(&folder.path().join("settings.json")).unwrap();
    assert_eq!(saved.pc_speaker_id.as_deref(), Some("speaker-a"));
    assert_eq!(saved.range, "far");
    assert!(saved.source_choice_done);
    change(&commands, Change::Mode("phone".into()))
        .await
        .unwrap();
    assert!(network.load(Ordering::Acquire));
    let (done, wait) = oneshot::channel();
    commands.send(Command::Shutdown(done)).await.unwrap();
    wait.await.unwrap();
    task.await.unwrap();
}
#[tokio::test]
async fn switching_during_preparation_cannot_start_old_capture_and_failed_saves_keep_the_active_choice(
) {
    let folder = tempfile::tempdir().unwrap();
    let live = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(AtomicBool::new(false));
    let (mut actor, network) = setup(
        folder.path(),
        live.clone(),
        started.clone(),
        Some(gate.clone()),
    );
    let bad = folder.path().join("blocked");
    std::fs::write(&bad, b"not a directory").unwrap();
    actor.path = bad.join("settings.json");
    assert!(actor.save(Change::Mode("pc".into())).is_err());
    assert_eq!(actor.snapshot().mode, "phone");
    assert_eq!(actor.settings.lock().unwrap().source, "phone");
    actor.path = folder.path().join("settings.json");
    let view = actor.view.clone();
    let source = actor.source.clone();
    let (commands, rx) = mpsc::channel(16);
    let (_devices, device_rx) = tokio::sync::watch::channel(devices());
    let task = tokio::spawn(actor.run(rx, device_rx));
    change(&commands, Change::Mode("pc".into())).await.unwrap();
    wait(&view, |v| v.state == "starting").await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while live.load(Ordering::Acquire) != 1 {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap();
    change(&commands, Change::Mode("phone".into()))
        .await
        .unwrap();
    gate.store(true, Ordering::Release);
    tokio::time::timeout(Duration::from_secs(2), async {
        while live.load(Ordering::Acquire) != 0 {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(started.load(Ordering::Acquire), 0);
    assert!(network.load(Ordering::Acquire));
    let generation = source.next_generation();
    change(&commands, Change::Devices(None, None))
        .await
        .unwrap();
    assert_eq!(
        source.next_generation(),
        generation + 1,
        "Editing PC choices while using the phone must not fence its stream."
    );
    let (done, wait) = oneshot::channel();
    commands.send(Command::Shutdown(done)).await.unwrap();
    wait.await.unwrap();
    task.await.unwrap();
}
