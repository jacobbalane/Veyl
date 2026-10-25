use crate::{
    connection::Command,
    events::{Diagnostics, PhoneTarget, PhonesView},
    Runtime,
};

#[tauri::command]
pub fn list_phones(state: tauri::State<'_, Runtime>) -> PhonesView {
    state.connection.snapshot().phones
}
#[tauri::command]
pub fn pair_phone(state: tauri::State<'_, Runtime>, target: PhoneTarget) -> Result<(), String> {
    state.connection.send(Command::Pair(target))
}
#[tauri::command]
pub fn confirm_pairing(
    state: tauri::State<'_, Runtime>,
    session_id: String,
    matches: bool,
) -> Result<(), String> {
    state.connection.send(Command::Confirm {
        session_id,
        matches,
    })
}
#[tauri::command]
pub fn connect_phone(state: tauri::State<'_, Runtime>, target: PhoneTarget) -> Result<(), String> {
    state.connection.send(Command::Connect(target))
}
#[tauri::command]
pub fn disconnect(state: tauri::State<'_, Runtime>) -> Result<(), String> {
    state.connection.send(Command::Disconnect)
}
#[tauri::command]
pub fn forget_phone(state: tauri::State<'_, Runtime>, phone_id: String) -> Result<(), String> {
    state.connection.send(Command::Forget(phone_id))
}
#[tauri::command]
pub fn request_phone_start(state: tauri::State<'_, Runtime>) -> Result<(), String> {
    state.connection.send(Command::RequestStart)
}
#[tauri::command]
pub fn get_diagnostics(state: tauri::State<'_, Runtime>) -> Diagnostics {
    let mut diagnostics = state.connection.diagnostics();
    diagnostics.source = Some(state.sources.snapshot());
    diagnostics
}
#[tauri::command]
pub fn set_actions_enabled(app: tauri::AppHandle, enabled: bool) {
    crate::set_actions(&app, enabled);
}
/// Validates and saves a settings change, then applies it to the engine, the actions, the hotkey
/// and starting with Windows. Detailed logs apply from the next start.
#[tauri::command]
pub async fn set_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, Runtime>,
    patch: crate::settings::SettingsPatch,
) -> Result<crate::settings::SettingsView, String> {
    use tauri_plugin_autostart::ManagerExt;
    let _change = state.settings_change.lock().await;
    let current = state.settings.lock().unwrap().clone();
    let preview = patch.clone().apply(&current)?;
    if preview.hotkey != current.hotkey {
        crate::replace_hotkey(&app, &current.hotkey, &preview.hotkey)?;
    }
    if preview.start_with_windows != current.start_with_windows {
        let autostart = app.autolaunch();
        let result = if preview.start_with_windows {
            autostart.enable()
        } else {
            autostart.disable()
        };
        if result.is_err() {
            if preview.hotkey != current.hotkey {
                let _ = crate::replace_hotkey(&app, &preview.hotkey, &current.hotkey);
            }
            return Err("autostart_failed".into());
        }
    }
    // Other parts of Veyl save the source and devices; apply the patch to the latest settings.
    let next = {
        let mut saved = state.settings.lock().unwrap();
        let next = patch.apply(&saved)?;
        next.save(&state.settings_path)
            .map_err(|_| "settings_save_failed")?;
        *saved = next.clone();
        next
    };
    state
        .actions
        .set_action(crate::actions::PokeAction::parse(&next.poke_action).unwrap_or_default());
    state
        .connection
        .engine()
        .apply(crate::engine_host::EngineSettings::from_settings(&next))
        .await?;
    Ok(crate::settings::SettingsView::from(&next))
}
/// The poke test and the onboarding practice: pokes are counted and actions never run.
#[tauri::command]
pub fn start_poke_test(state: tauri::State<'_, Runtime>) {
    state.connection.engine().start_test();
}
#[tauri::command]
pub fn stop_poke_test(state: tauri::State<'_, Runtime>) -> crate::engine_host::PokeTestResult {
    state.connection.engine().stop_test()
}
/// The interface's echo view; a reloaded window replaces the previous channel.
#[tauri::command]
pub fn subscribe_live(
    state: tauri::State<'_, Runtime>,
    channel: tauri::ipc::Channel<crate::events::LiveFrame>,
) {
    state
        .connection
        .engine()
        .subscribe(Box::new(move |frame| channel.send(frame.clone()).is_ok()));
}
#[tauri::command]
pub fn copy_diagnostics(state: tauri::State<'_, Runtime>) -> Result<String, String> {
    serde_json::to_string_pretty(&get_diagnostics(state))
        .map_err(|_| "Veyl could not copy diagnostics.".into())
}

#[tauri::command]
pub async fn set_source(state: tauri::State<'_, Runtime>, source: String) -> Result<(), String> {
    state.sources.set_source(source).await?;
    // Switching to the PC may widen the distance range.
    let settings =
        crate::engine_host::EngineSettings::from_settings(&state.settings.lock().unwrap());
    state.connection.engine().apply(settings).await
}
#[tauri::command]
pub async fn set_pc_devices(
    state: tauri::State<'_, Runtime>,
    speaker_id: Option<String>,
    microphone_id: Option<String>,
) -> Result<(), String> {
    state.sources.set_devices(speaker_id, microphone_id).await
}
#[tauri::command]
pub fn list_audio_devices(state: tauri::State<'_, Runtime>) -> crate::pc_devices::AudioDevices {
    state.sources.snapshot().devices
}
#[tauri::command]
pub async fn check_pc_sound(state: tauri::State<'_, Runtime>) -> Result<(), String> {
    state.sources.check_sound().await
}
#[tauri::command]
pub async fn cancel_pc_sound_check(state: tauri::State<'_, Runtime>) -> Result<(), String> {
    state.sources.cancel_check().await
}

#[tauri::command]
pub async fn start_recording(
    state: tauri::State<'_, Runtime>,
    seconds: u32,
    mark_pokes: bool,
) -> Result<crate::recorder::RecordingInfo, String> {
    let link = state.connection.snapshot();
    let saved = link
        .phones
        .paired
        .iter()
        .find(|phone| Some(&phone.phone_id) == link.connection.phone_id.as_ref());
    state.recorder.start(seconds, mark_pokes, serde_json::json!({
        "phone_name": link.connection.phone_name, "model": saved.map(|phone| phone.model.as_str()),
    })).await
}
#[tauri::command]
pub async fn stop_recording(
    state: tauri::State<'_, Runtime>,
) -> Result<crate::recorder::RecordingInfo, String> {
    state.recorder.stop().await
}
#[tauri::command]
pub fn mark_poke(
    state: tauri::State<'_, Runtime>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    if window.label() == "main" && window.is_focused().unwrap_or(false) {
        state.recorder.mark()
    } else {
        Ok(())
    }
}
#[tauri::command]
pub fn get_recording(state: tauri::State<'_, Runtime>) -> Option<crate::recorder::RecordingInfo> {
    state.recorder.snapshot()
}
#[tauri::command]
pub fn list_recordings(
    state: tauri::State<'_, Runtime>,
) -> Result<Vec<crate::recorder::RecordingInfo>, String> {
    state.recorder.list().map_err(|_| "read_failed".into())
}
#[tauri::command]
pub fn delete_recording(state: tauri::State<'_, Runtime>, id: String) -> Result<(), String> {
    state
        .recorder
        .delete(&id)
        .map_err(|_| "delete_failed".into())
}
#[tauri::command]
pub fn open_recordings_folder(state: tauri::State<'_, Runtime>) -> Result<(), String> {
    open_folder(state.recorder.root())
}
#[tauri::command]
pub fn open_logs_folder(state: tauri::State<'_, Runtime>) -> Result<(), String> {
    open_folder(&state.logs)
}
fn open_folder(path: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(explorer_path(path))
            .spawn()
            .map_err(|_| "open_failed".to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("open_failed".into())
    }
}

#[cfg(windows)]
fn explorer_path(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::{Component, PathBuf, Prefix};
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return path.to_owned();
    };
    // Canonical filesystem paths use a prefix that Explorer does not accept.
    let mut visible = match prefix.kind() {
        Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", char::from(drive))),
        Prefix::VerbatimUNC(server, share) => {
            let mut prefix = std::ffi::OsString::from("\\\\");
            prefix.push(server);
            prefix.push("\\");
            prefix.push(share);
            PathBuf::from(prefix)
        }
        _ => return path.to_owned(),
    };
    for component in components {
        visible.push(component.as_os_str());
    }
    visible
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn explorer_accepts_canonical_disk_and_unc_folders_without_verbatim_prefixes() {
        for (original, expected) in [
            (
                r"\\?\C:\Users\Test name\recordings",
                r"C:\Users\Test name\recordings",
            ),
            (
                r"\\?\UNC\server\share\recordings",
                r"\\server\share\recordings",
            ),
        ] {
            assert_eq!(
                explorer_path(std::path::Path::new(original)),
                std::path::PathBuf::from(expected)
            );
        }
    }
}
