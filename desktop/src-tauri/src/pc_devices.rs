//! Endpoint metadata never opens an audio capture stream. Native COM stays on its own thread.
use serde::Serialize;
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::{oneshot, watch};

#[cfg(windows)]
mod native;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub form_factor: String,
    pub raw_supported: bool,
    pub bluetooth: bool,
    pub built_in: bool,
}
impl AudioDevice {
    pub fn headphones(&self) -> bool {
        matches!(self.form_factor.as_str(), "headphones" | "headset")
    }
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevices {
    pub speakers: Vec<AudioDevice>,
    pub microphones: Vec<AudioDevice>,
    pub default_speaker_id: Option<String>,
    pub default_microphone_id: Option<String>,
    pub has_battery: Option<bool>,
    pub recommended_source: String,
    pub revision: u64,
    pub error: Option<String>,
}
impl Default for AudioDevices {
    fn default() -> Self {
        Self {
            speakers: vec![],
            microphones: vec![],
            default_speaker_id: None,
            default_microphone_id: None,
            has_battery: None,
            recommended_source: "phone".into(),
            revision: 0,
            error: Some("loading".into()),
        }
    }
}
impl AudioDevices {
    pub fn speaker(&self, id: Option<&str>) -> Option<&AudioDevice> {
        let id = id.or(self.default_speaker_id.as_deref())?;
        self.speakers.iter().find(|device| device.id == id)
    }
    pub fn microphone(&self, id: Option<&str>) -> Option<&AudioDevice> {
        let id = id.or(self.default_microphone_id.as_deref())?;
        self.microphones.iter().find(|device| device.id == id)
    }
    fn recommend(&mut self) {
        let compatible = self
            .speakers
            .iter()
            .filter(|device| !device.headphones())
            .any(|speaker| {
                self.microphones
                    .iter()
                    .any(|mic| self.has_battery == Some(true) || speaker.built_in && mic.built_in)
            });
        self.recommended_source = if compatible { "pc" } else { "phone" }.into();
    }
}
struct Monitor {
    view: watch::Sender<AudioDevices>,
    stop: Arc<AtomicBool>,
    wake: SyncSender<()>,
    done: Mutex<Option<oneshot::Receiver<()>>>,
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.wake.try_send(());
    }
}
#[derive(Clone)]
pub struct DeviceMonitor(Arc<Monitor>);
impl DeviceMonitor {
    pub fn start() -> io::Result<Self> {
        let (view, _) = watch::channel(AudioDevices::default());
        let stop = Arc::new(AtomicBool::new(false));
        let (wake, notices) = mpsc::sync_channel(1);
        let (done, finished) = oneshot::channel();
        let monitor = Self(Arc::new(Monitor {
            view: view.clone(),
            stop: stop.clone(),
            wake: wake.clone(),
            done: Mutex::new(Some(finished)),
        }));
        std::thread::Builder::new()
            .name("Veyl-devices".into())
            .spawn(move || {
                #[cfg(windows)]
                native::watch(view, stop, wake, notices);
                #[cfg(not(windows))]
                {
                    let _ = (stop, wake, notices);
                    view.send_replace(AudioDevices {
                        error: Some("windows_only".into()),
                        ..Default::default()
                    });
                }
                let _ = done.send(());
            })?;
        Ok(monitor)
    }
    pub fn snapshot(&self) -> AudioDevices {
        self.0.view.borrow().clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<AudioDevices> {
        self.0.view.subscribe()
    }
    pub async fn shutdown(&self) {
        self.0.stop.store(true, Ordering::Release);
        let _ = self.0.wake.try_send(());
        let done = self.0.done.lock().unwrap().take();
        if let Some(done) = done {
            let _ = tokio::time::timeout(Duration::from_secs(2), done).await;
        }
    }
}

fn form_factor(value: u32) -> &'static str {
    match value {
        0 => "remote",
        1 => "speakers",
        2 => "line",
        3 => "headphones",
        4 => "microphone",
        5 => "headset",
        6 => "handset",
        7 => "digital",
        8 => "spdif",
        9 => "hdmi",
        _ => "unknown",
    }
}
fn transport(enumerator: Option<&str>) -> (bool, bool) {
    match enumerator.map(str::to_ascii_uppercase).as_deref() {
        Some("BTHENUM" | "BTHLEDEVICE") => (true, false),
        // Bus identifiers describe hardware integration; endpoint names/IDs remain opaque.
        Some("HDAUDIO" | "PCI" | "ACPI") => (false, true),
        _ => (false, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn device(id: &str, form: &str, built_in: bool) -> AudioDevice {
        AudioDevice {
            id: id.into(),
            name: "Device".into(),
            form_factor: form.into(),
            raw_supported: false,
            bluetooth: false,
            built_in,
        }
    }
    #[test]
    fn mode_recommendations_require_speakers_and_a_mic_and_do_not_guess_unknown_hardware() {
        let mut devices = AudioDevices {
            speakers: vec![device("speaker", "speakers", true)],
            microphones: vec![device("mic", "microphone", true)],
            ..Default::default()
        };
        devices.recommend();
        assert_eq!(devices.recommended_source, "pc");
        devices.microphones[0].built_in = false;
        devices.recommend();
        assert_eq!(devices.recommended_source, "phone");
        devices.has_battery = Some(true);
        devices.recommend();
        assert_eq!(devices.recommended_source, "pc");
        devices.speakers[0].form_factor = "headphones".into();
        devices.recommend();
        assert_eq!(devices.recommended_source, "phone");
        devices.speakers[0].form_factor = "speakers".into();
        devices.microphones.clear();
        devices.recommend();
        assert_eq!(devices.recommended_source, "phone");
    }
    #[test]
    fn default_following_preserves_opaque_case_sensitive_ids_and_never_substitutes_missing_choices()
    {
        let devices = AudioDevices {
            speakers: vec![device("Case-Sensitive-ID", "speakers", true)],
            default_speaker_id: Some("Case-Sensitive-ID".into()),
            ..Default::default()
        };
        assert!(devices.speaker(None).is_some());
        assert!(devices.speaker(Some("case-sensitive-id")).is_none());
        assert!(devices.speaker(Some("removed-device")).is_none());
        assert!(devices.microphone(None).is_none());
        assert_eq!(form_factor(3), "headphones");
        assert_eq!(form_factor(5), "headset");
        assert_eq!(form_factor(u32::MAX), "unknown");
        assert_eq!(transport(Some("bthledevice")), (true, false));
        assert_eq!(transport(Some("BTHENUM")), (true, false));
        assert_eq!(transport(Some("HDAUDIO")), (false, true));
        assert_eq!(transport(Some("USB")), (false, false));
        assert_eq!(transport(None), (false, false));
    }
    #[tokio::test]
    async fn native_metadata_monitor_publishes_a_snapshot_or_an_explicit_error_and_shuts_down() {
        let monitor = DeviceMonitor::start().unwrap();
        let mut updates = monitor.subscribe();
        tokio::time::timeout(Duration::from_secs(5), async {
            while updates.borrow().error.as_deref() == Some("loading") {
                updates.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
        let snapshot = monitor.snapshot();
        assert_ne!(snapshot.error.as_deref(), Some("loading"));
        monitor.shutdown().await;
    }
}
