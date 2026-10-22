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
pub enum Notice {
    Connection(ConnectionState),
    Phones(PhonesView),
    Pairing(PairingEvent),
    Capture(Capture),
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
        for (name, value) in [
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
