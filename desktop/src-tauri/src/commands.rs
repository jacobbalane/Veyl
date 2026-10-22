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
