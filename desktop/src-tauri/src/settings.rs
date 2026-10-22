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
}
