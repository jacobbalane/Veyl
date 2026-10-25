use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::Path,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    pub pc_id: String,
    pub pc_name: String,
    pub poke_action: String,
    pub sensitivity: String,
    pub range: String,
    pub hotkey: String,
    pub actions_on_at_start: bool,
    pub start_with_windows: bool,
    pub close_to_tray: bool,
    pub detailed_logs: bool,
    pub keep_recordings_days: u32,
    pub temperature_c: f32,
    pub onboarding_done: bool,
    pub last_phone_id: Option<String>,
    pub source: String,
    #[serde(default)]
    pub source_choice_done: bool,
    pub pc_speaker_id: Option<String>,
    pub pc_mic_id: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

impl Settings {
    fn new() -> io::Result<Self> {
        let mut id = [0; 16];
        getrandom::fill(&mut id).map_err(io::Error::other)?;
        let mut name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Windows PC".into());
        while name.len() > 40 {
            name.pop();
        }
        Ok(Self {
            version: 1,
            pc_id: URL_SAFE_NO_PAD.encode(id),
            pc_name: name,
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
            last_phone_id: None,
            source: "phone".into(),
            source_choice_done: false,
            pc_speaker_id: None,
            pc_mic_id: None,
            extra: BTreeMap::new(),
        })
    }

    pub fn load_or_create(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(bytes) => {
                let settings: Self = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
                if settings.version != 1
                    || URL_SAFE_NO_PAD
                        .decode(&settings.pc_id)
                        .map_or(true, |id| id.len() != 16)
                {
                    return Err(io::Error::other("Unsupported or invalid Veyl settings."));
                }
                Ok(settings)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let settings = Self::new()?;
                settings.save(path)?;
                Ok(settings)
            }
            Err(error) => Err(error),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let directory = path
            .parent()
            .ok_or_else(|| io::Error::other("Settings need a parent directory."))?;
        fs::create_dir_all(directory)?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
        serde_json::to_writer_pretty(&mut temporary, self).map_err(io::Error::other)?;
        temporary.write_all(b"\n")?;
        temporary.as_file().sync_all()?;
        temporary.persist(path).map_err(|error| error.error)?;
        Ok(())
    }
}

/// The settings the interface shows and changes.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub pc_name: String,
    pub poke_action: String,
    pub sensitivity: String,
    pub range: String,
    pub hotkey: String,
    pub actions_on_at_start: bool,
    pub start_with_windows: bool,
    pub close_to_tray: bool,
    pub detailed_logs: bool,
    pub keep_recordings_days: u32,
    pub temperature_c: f32,
    pub onboarding_done: bool,
}
impl From<&Settings> for SettingsView {
    fn from(settings: &Settings) -> Self {
        Self {
            pc_name: settings.pc_name.clone(),
            poke_action: settings.poke_action.clone(),
            sensitivity: settings.sensitivity.clone(),
            range: settings.range.clone(),
            hotkey: settings.hotkey.clone(),
            actions_on_at_start: settings.actions_on_at_start,
            start_with_windows: settings.start_with_windows,
            close_to_tray: settings.close_to_tray,
            detailed_logs: settings.detailed_logs,
            keep_recordings_days: settings.keep_recordings_days,
            temperature_c: settings.temperature_c,
            onboarding_done: settings.onboarding_done,
        }
    }
}
/// A change from the interface; missing fields stay as they are.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsPatch {
    pub pc_name: Option<String>,
    pub poke_action: Option<String>,
    pub sensitivity: Option<String>,
    pub range: Option<String>,
    pub hotkey: Option<String>,
    pub actions_on_at_start: Option<bool>,
    pub start_with_windows: Option<bool>,
    pub close_to_tray: Option<bool>,
    pub detailed_logs: Option<bool>,
    pub keep_recordings_days: Option<u32>,
    pub temperature_c: Option<f32>,
    pub onboarding_done: Option<bool>,
}
impl SettingsPatch {
    /// The settings with this change, or the code of the first invalid value.
    pub fn apply(self, settings: &Settings) -> Result<Settings, &'static str> {
        let mut next = settings.clone();
        if let Some(name) = self.pc_name {
            let name = name.trim();
            if name.is_empty() || name.len() > 40 || name.chars().any(char::is_control) {
                return Err("invalid_pc_name");
            }
            next.pc_name = name.into();
        }
        if let Some(action) = self.poke_action {
            crate::actions::PokeAction::parse(&action).ok_or("invalid_poke_action")?;
            next.poke_action = action;
        }
        if let Some(sensitivity) = self.sensitivity {
            if !matches!(sensitivity.as_str(), "low" | "normal" | "high") {
                return Err("invalid_sensitivity");
            }
            next.sensitivity = sensitivity;
        }
        if let Some(range) = self.range {
            if !matches!(range.as_str(), "near" | "normal" | "far") {
                return Err("invalid_range");
            }
            next.range = range;
        }
        if let Some(hotkey) = self.hotkey {
            if hotkey.trim().is_empty() || hotkey.len() > 40 {
                return Err("invalid_hotkey");
            }
            next.hotkey = hotkey.trim().into();
        }
        if let Some(days) = self.keep_recordings_days {
            // Zero keeps recordings until the user deletes them.
            if days > 365 {
                return Err("invalid_keep_days");
            }
            next.keep_recordings_days = days;
        }
        if let Some(temperature) = self.temperature_c {
            if !temperature.is_finite() || !(-10.0..=45.0).contains(&temperature) {
                return Err("invalid_temperature");
            }
            next.temperature_c = temperature;
        }
        next.actions_on_at_start = self.actions_on_at_start.unwrap_or(next.actions_on_at_start);
        next.start_with_windows = self.start_with_windows.unwrap_or(next.start_with_windows);
        next.close_to_tray = self.close_to_tray.unwrap_or(next.close_to_tray);
        next.detailed_logs = self.detailed_logs.unwrap_or(next.detailed_logs);
        next.onboarding_done = self.onboarding_done.unwrap_or(next.onboarding_done);
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_survives_restart_and_unknown_settings_survive_saves() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("settings.json");
        let mut first = Settings::load_or_create(&path).unwrap();
        assert_eq!(URL_SAFE_NO_PAD.decode(&first.pc_id).unwrap().len(), 16);
        assert!(!first.actions_on_at_start);
        first
            .extra
            .insert("future_setting".into(), serde_json::json!(42));
        first.save(&path).unwrap();
        let loaded = Settings::load_or_create(&path).unwrap();
        assert_eq!(first.pc_id, loaded.pc_id);
        assert_eq!(loaded.extra["future_setting"], 42);
        fs::write(&path, b"invalid").unwrap();
        assert!(Settings::load_or_create(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"invalid");
    }
    #[test]
    fn patches_validate_each_value_and_keep_the_rest() {
        let folder = tempfile::tempdir().unwrap();
        let settings = Settings::load_or_create(&folder.path().join("settings.json")).unwrap();
        let patch: SettingsPatch = serde_json::from_value(serde_json::json!({
            "pokeAction": "play_pause",
            "sensitivity": "high",
            "temperatureC": 26.5,
            "pcName": "  Desk PC ",
        }))
        .unwrap();
        let next = patch.apply(&settings).unwrap();
        assert_eq!(next.poke_action, "play_pause");
        assert_eq!(next.sensitivity, "high");
        assert_eq!(next.temperature_c, 26.5);
        assert_eq!(next.pc_name, "Desk PC");
        assert_eq!(next.range, settings.range);
        assert_eq!(next.pc_id, settings.pc_id);
        for (patch, code) in [
            (
                serde_json::json!({"pokeAction": "jump"}),
                "invalid_poke_action",
            ),
            (serde_json::json!({"range": "huge"}), "invalid_range"),
            (
                serde_json::json!({"temperatureC": 80.0}),
                "invalid_temperature",
            ),
            (serde_json::json!({"pcName": " "}), "invalid_pc_name"),
            (
                serde_json::json!({"keepRecordingsDays": 999}),
                "invalid_keep_days",
            ),
        ] {
            let patch: SettingsPatch = serde_json::from_value(patch).unwrap();
            assert_eq!(patch.apply(&settings).unwrap_err(), code);
        }
        assert!(serde_json::from_value::<SettingsPatch>(serde_json::json!({"pcId": "x"})).is_err());
        let view = SettingsView::from(&next);
        let json = serde_json::to_value(view).unwrap();
        assert_eq!(json["pokeAction"], "play_pause");
        assert!(json.get("pcId").is_none());
    }
}
