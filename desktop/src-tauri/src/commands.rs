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
    state.connection.diagnostics()
}
#[tauri::command]
pub fn copy_diagnostics(state: tauri::State<'_, Runtime>) -> Result<String, String> {
    serde_json::to_string_pretty(&state.connection.diagnostics())
        .map_err(|_| "Veyl could not copy diagnostics.".into())
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
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(explorer_path(state.recorder.root()))
            .spawn()
            .map_err(|_| "open_failed".to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = state;
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
