use crate::{discovery::NearbyPhone, engine_host::LevelsView, pairings::SavedPhone};
use serde::{Deserialize, Serialize};
use veyl_link::messages::Capture;

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum PhoneTarget {
    Nearby {
        #[serde(rename = "nearbyId")]
        nearby_id: String,
    },
    Address {
        address: String,
    },
    Saved {
        #[serde(rename = "phoneId")]
        phone_id: String,
    },
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhonesView {
    pub paired: Vec<SavedPhone>,
    pub nearby: Vec<NearbyPhone>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionState {
    pub state: String,
    pub phone_id: Option<String>,
    pub phone_name: Option<String>,
    pub address: Option<String>,
    pub capture: Option<Capture>,
    pub error: Option<String>,
    pub attempt: u32,
}
impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            state: "browsing".into(),
            phone_id: None,
            phone_name: None,
            address: None,
            capture: None,
            error: None,
            attempt: 0,
        }
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingEvent {
    pub session_id: String,
    pub phone_name: String,
    pub code: Option<String>,
    pub stage: String,
    pub error: Option<String>,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneStats {
    pub battery_pct: Option<u8>,
    pub charging: bool,
    pub queue_ms: u32,
    pub dropped_blocks: u64,
    pub media_volume: Option<f32>,
    pub thermal: Option<String>,
    pub cpu_pct: Option<f32>,
    pub rtt_ms: Option<f64>,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkSnapshot {
    pub connection: ConnectionState,
    pub phones: PhonesView,
    pub pairing: Option<PairingEvent>,
    pub phone_stats: PhoneStats,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub link: LinkSnapshot,
    pub levels: LevelsView,
    pub source: Option<crate::sources::SourceView>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandView {
    pub present: bool,
    pub distance_cm: Option<f32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeterView {
    pub score: u8,
    pub label: veyl_dsp::MeterLabel,
    pub high: f32,
    pub detail: f32,
    pub volume: f32,
    pub steady: f32,
    pub clarity: f32,
    pub detail_cm: Option<f32>,
    pub tip: veyl_dsp::Tip,
}
impl From<&veyl_dsp::MeterReading> for MeterView {
    fn from(reading: &veyl_dsp::MeterReading) -> Self {
        Self {
            score: reading.score,
            label: reading.label,
            high: reading.factors.high,
            detail: reading.factors.detail,
            volume: reading.factors.volume,
            steady: reading.factors.steady,
            clarity: reading.factors.clarity,
            detail_cm: reading.detail_cm,
            tip: reading.tip,
        }
    }
}
/// One column of the echo view, about 30 per second; the meter rides along about 10 times a second.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveFrame {
    pub t: f64,
    pub state: veyl_dsp::EngineState,
    pub motion: Vec<f32>,
    pub hand: HandView,
    pub meter: Option<MeterView>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineView {
    pub state: veyl_dsp::EngineState,
    /// Why pokes are not detected right now: `paused`, `source`, `starting`, `sound` or `settling`.
    pub suspended: Option<String>,
    /// A recent engine warning: `relocked`, `model_jump`, `impulsive_noise` or `drift`.
    pub warning: Option<String>,
    pub hand: HandView,
    pub meter: Option<MeterView>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GestureEvent {
    pub kind: String,
    pub at: f64,
    pub distance_cm: f32,
    pub depth_cm: f32,
    pub confidence: f32,
    pub acted: bool,
    pub test_only: bool,
}
/// 'veyl://actions': the switch changed, or Windows blocked an action.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionsEvent {
    pub enabled: bool,
    /// `blocked_elevated` when the target app runs as administrator.
    pub error: Option<String>,
}
pub enum Notice {
    Connection(ConnectionState),
    Phones(PhonesView),
    Pairing(PairingEvent),
    Capture(Capture),
    Engine(EngineView),
    Gesture(GestureEvent),
}

#[cfg(test)]
mod fixtures {
    use super::*;
    #[test]
    fn rust_payloads_match_committed_interface_fixtures() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/api/fixtures");
        let app = crate::AppState {
            version: env!("CARGO_PKG_VERSION"),
            pc_name: "Test PC".into(),
            actions_enabled: false,
            link: LinkSnapshot::default(),
            source: crate::sources::SourceView::default(),
            engine: EngineView::default(),
            settings: crate::settings::SettingsView {
                pc_name: "Test PC".into(),
                poke_action: "click".into(),
                sensitivity: "normal".into(),
                range: "normal".into(),
                hotkey: "Ctrl+Alt+V".into(),
                actions_on_at_start: false,
                start_with_windows: false,
                close_to_tray: true,
                detailed_logs: false,
                keep_recordings_days: 7,
                temperature_c: 20.0,
                onboarding_done: false,
            },
        };
        let diagnostics = Diagnostics {
            link: LinkSnapshot::default(),
            levels: LevelsView::default(),
            source: Some(crate::sources::SourceView::default()),
        };
        let pairing = PairingEvent {
            session_id: "test-comparison".into(),
            phone_name: "Test phone".into(),
            code: Some("123456".into()),
            stage: "compare".into(),
            error: None,
        };
        let meter = MeterView {
            score: 68,
            label: veyl_dsp::MeterLabel::Good,
            high: 0.75,
            detail: 0.5,
            volume: 1.0,
            steady: 0.875,
            clarity: 0.625,
            detail_cm: Some(1.75),
            tip: veyl_dsp::Tip::FewHighSounds,
        };
        let hand = HandView {
            present: true,
            distance_cm: Some(18.5),
        };
        let engine = EngineView {
            state: veyl_dsp::EngineState::Running,
            suspended: Some("settling".into()),
            warning: Some("relocked".into()),
            hand,
            meter: Some(meter),
        };
        let mut motion = vec![0.0; 60];
        motion[18] = 24.5;
        let live = LiveFrame {
            t: 1250.5,
            state: veyl_dsp::EngineState::Running,
            motion,
            hand,
            meter: Some(meter),
        };
        let gesture = GestureEvent {
            kind: "poke".into(),
            at: 1300.25,
            distance_cm: 14.5,
            depth_cm: 4.25,
            confidence: 0.875,
            acted: false,
            test_only: true,
        };
        for (name, value) in [
            ("engine.json", serde_json::to_value(engine).unwrap()),
            ("live-frame.json", serde_json::to_value(live).unwrap()),
            ("gesture.json", serde_json::to_value(gesture).unwrap()),
            (
                "actions.json",
                serde_json::to_value(ActionsEvent {
                    enabled: true,
                    error: Some("blocked_elevated".into()),
                })
                .unwrap(),
            ),
            (
                "poke-test.json",
                serde_json::to_value(crate::engine_host::PokeTestResult { seen: 8, of: 10 })
                    .unwrap(),
            ),
            (
                "pc-check.json",
                serde_json::to_value(crate::pc_testsound::CheckResult {
                    good: true,
                    mean_coherence: 0.75,
                    coverage: 0.5,
                    bandwidth_hz: 10000.0,
                    lag_ms: 12.0,
                })
                .unwrap(),
            ),
            ("app-state.json", serde_json::to_value(app).unwrap()),
            (
                "diagnostics.json",
                serde_json::to_value(diagnostics).unwrap(),
            ),
            ("pairing.json", serde_json::to_value(pairing).unwrap()),
            (
                "source.json",
                serde_json::to_value(crate::sources::SourceView::default()).unwrap(),
            ),
            (
                "recording.json",
                serde_json::to_value(crate::recorder::RecordingInfo {
                    id: "20261022-120000-00000000".into(),
                    seconds: 30,
                    mark_pokes: true,
                    state: "recording".into(),
                    remaining_seconds: 20,
                    blocks: [1000, 1000],
                    error: None,
                })
                .unwrap(),
            ),
        ] {
            let expected = format!("{}\n", serde_json::to_string_pretty(&value).unwrap());
            let path = root.join(name);
            if std::env::var_os("VEYL_UPDATE_FIXTURES").is_some() {
                std::fs::create_dir_all(&root).unwrap();
                std::fs::write(&path, &expected).unwrap();
            }
            let actual: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(path)
                    .expect("Regenerate interface fixtures after an intentional API change."),
            )
            .unwrap();
            assert_eq!(actual, value, "{name}");
        }
    }
}
