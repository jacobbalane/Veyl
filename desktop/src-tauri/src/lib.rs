pub mod actions;
pub mod address;
mod commands;
pub mod connection;
pub mod discovery;
pub mod engine_host;
pub mod events;
pub mod pairings;
pub mod pc_devices;
pub mod pc_source;
pub mod pc_testsound;
pub mod recorder;
mod settings;
pub mod source;
pub mod sources;

use settings::Settings;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

struct Runtime {
    settings: Arc<Mutex<Settings>>,
    settings_path: std::path::PathBuf,
    logs: std::path::PathBuf,
    /// Serializes settings changes from the interface.
    settings_change: tokio::sync::Mutex<()>,
    actions: Arc<actions::ActionsGate>,
    tray: TrayIcon,
    tray_actions: CheckMenuItem<tauri::Wry>,
    connection: connection::ConnectionManager,
    recorder: recorder::Recorder,
    sources: sources::Sources,
    recording_events_stop: tokio::sync::watch::Sender<bool>,
    exiting: AtomicBool,
    hidden: bool,
    _logging: tracing_appender::non_blocking::WorkerGuard,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppState {
    version: &'static str,
    pc_name: String,
    actions_enabled: bool,
    link: events::LinkSnapshot,
    source: sources::SourceView,
    engine: events::EngineView,
    settings: settings::SettingsView,
}

#[tauri::command]
fn get_app_state(state: tauri::State<'_, Runtime>) -> AppState {
    let settings = settings::SettingsView::from(&*state.settings.lock().unwrap());
    AppState {
        version: env!("CARGO_PKG_VERSION"),
        pc_name: settings.pc_name.clone(),
        actions_enabled: state.actions.enabled(),
        link: state.connection.snapshot(),
        source: state.sources.snapshot(),
        engine: state.connection.engine().engine(),
        settings,
    }
}

/// Turns actions on or off from the header, the tray or the hotkey, and shows it everywhere.
fn set_actions(app: &tauri::AppHandle, enabled: bool) {
    let Some(runtime) = app.try_state::<Runtime>() else {
        return;
    };
    let changed = runtime.actions.set_enabled(enabled);
    let _ = runtime.tray_actions.set_checked(enabled);
    let _ = runtime.tray.set_tooltip(Some(tray_tooltip(enabled)));
    let _ = app.emit(
        "veyl://actions",
        events::ActionsEvent {
            enabled,
            error: None,
        },
    );
    if changed {
        tracing::info!("Actions {}.", if enabled { "on" } else { "off" });
    }
}
fn toggle_actions(app: &tauri::AppHandle) {
    let enabled = app
        .try_state::<Runtime>()
        .is_some_and(|runtime| runtime.actions.enabled());
    set_actions(app, !enabled);
}
fn tray_tooltip(actions: bool) -> &'static str {
    if actions {
        "Veyl · Actions on"
    } else {
        "Veyl · Actions off"
    }
}
fn parse_hotkey(hotkey: &str) -> Result<Shortcut, String> {
    hotkey.parse().map_err(|_| "invalid_hotkey".to_string())
}
/// Moves the actions hotkey; on failure the previous one stays registered.
fn replace_hotkey(app: &tauri::AppHandle, previous: &str, next: &str) -> Result<(), String> {
    let next = parse_hotkey(next)?;
    let shortcuts = app.global_shortcut();
    let previous = parse_hotkey(previous).ok();
    if let Some(previous) = previous {
        let _ = shortcuts.unregister(previous);
    }
    if shortcuts.register(next).is_err() {
        if let Some(previous) = previous {
            let _ = shortcuts.register(previous);
        }
        return Err("hotkey_unavailable".into());
    }
    Ok(())
}

#[tauri::command]
fn ui_ready(app: tauri::AppHandle, state: tauri::State<'_, Runtime>) -> Result<(), String> {
    if !state.hidden {
        let window = app
            .get_webview_window("main")
            .ok_or("Veyl's window is missing.")?;
        window.show().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_window(app)
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_actions(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let directory = if cfg!(debug_assertions) {
                std::env::var_os("VEYL_DATA_DIR").map(std::path::PathBuf::from)
            } else {
                None
            }
            .unwrap_or(app.path().app_local_data_dir()?);
            std::fs::create_dir_all(&directory)?;
            let settings_path = directory.join("settings.json");
            let settings = Arc::new(Mutex::new(Settings::load_or_create(&settings_path)?));
            let log = tracing_appender::rolling::Builder::new()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("veyl")
                .filename_suffix("log")
                .max_log_files(7)
                .build(directory.join("logs"))?;
            let logs = directory.join("logs");
            let (writer, guard) = tracing_appender::non_blocking(log);
            tracing_subscriber::fmt()
                .with_ansi(false)
                // Detailed logs apply from the next start.
                .with_max_level(if settings.lock().unwrap().detailed_logs {
                    tracing::Level::DEBUG
                } else {
                    tracing::Level::INFO
                })
                .with_writer(writer)
                .try_init()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let handle = app.handle().clone();
            let recorder = recorder::Recorder::new(
                &directory.join("recordings"),
                settings.lock().unwrap().keep_recordings_days,
            )?;
            let mut recordings = recorder.subscribe();
            let recording_events = app.handle().clone();
            let (recording_events_stop, mut stop_recording_events) = tokio::sync::watch::channel(false);
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    let changed = tokio::select! { _ = stop_recording_events.changed() => break, changed = recordings.changed() => changed };
                    if changed.is_err() { break; }
                    tokio::select! { _ = stop_recording_events.changed() => break, _ = interval.tick() => {} }
                    let info = recordings.borrow_and_update().clone();
                    let _ = recording_events.emit("veyl://recording", info);
                }
            });
            let (actions_on, poke_action, hotkey) = {
                let saved = settings.lock().unwrap();
                (
                    saved.actions_on_at_start,
                    actions::PokeAction::parse(&saved.poke_action).unwrap_or_default(),
                    saved.hotkey.clone(),
                )
            };
            let blocked = app.handle().clone();
            let actions = actions::ActionsGate::new(
                actions_on,
                poke_action,
                Arc::new(actions::SystemActions),
                Arc::new(move |_, result| {
                    if result == Err(actions::ActionError::Blocked) {
                        let enabled = blocked
                            .try_state::<Runtime>()
                            .is_some_and(|runtime| runtime.actions.enabled());
                        let _ = blocked.emit(
                            "veyl://actions",
                            events::ActionsEvent {
                                enabled,
                                error: Some("blocked_elevated".into()),
                            },
                        );
                    }
                }),
            );
            let connection = connection::ConnectionManager::start(
                settings.clone(),
                &directory,
                Arc::new(move |notice| match notice {
                    events::Notice::Connection(value) => {
                        let _ = handle.emit("veyl://connection", value);
                    }
                    events::Notice::Phones(value) => {
                        let _ = handle.emit("veyl://phones", value);
                    }
                    events::Notice::Pairing(value) => {
                        let _ = handle.emit("veyl://pairing", value);
                    }
                    events::Notice::Capture(value) => {
                        let _ = handle.emit("veyl://capture", value);
                    }
                    events::Notice::Engine(value) => {
                        let _ = handle.emit("veyl://engine", value);
                    }
                    events::Notice::Gesture(value) => {
                        let _ = handle.emit("veyl://gesture", value);
                    }
                }),
                recorder.clone(),
                actions.clone(),
            )?;
            let source_events = app.handle().clone();
            let sources = sources::Sources::start(settings.clone(), &directory, connection.clone(), pc_devices::DeviceMonitor::start()?, Arc::new(move |source| { let _ = source_events.emit("veyl://source", source); }));
            let open = MenuItem::with_id(app, "open", "Open Veyl", true, None::<&str>)?;
            let tray_actions =
                CheckMenuItem::with_id(app, "actions", "Actions", true, actions_on, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Veyl", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &tray_actions, &separator, &quit])?;
            let icon = app
                .default_window_icon()
                .ok_or("Veyl's icon is missing.")?
                .clone();
            let tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip(tray_tooltip(actions_on))
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_window(app),
                    "actions" => toggle_actions(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;
            app.manage(Runtime {
                settings,
                settings_path,
                logs,
                settings_change: tokio::sync::Mutex::new(()),
                actions,
                tray,
                tray_actions,
                connection,
                recorder,
                sources,
                recording_events_stop,
                exiting: AtomicBool::new(false),
                hidden: std::env::args().any(|arg| arg == "--hidden"),
                _logging: guard,
            });
            // Another app may own the hotkey; the switch and the tray still work.
            match parse_hotkey(&hotkey) {
                Ok(shortcut) if app.global_shortcut().register(shortcut).is_ok() => {}
                _ => tracing::warn!("The actions hotkey is unavailable."),
            }
            tracing::info!(
                "Veyl started with actions {}.",
                if actions_on { "on" } else { "off" }
            );
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window
                    .state::<Runtime>()
                    .settings
                    .lock()
                    .unwrap()
                    .close_to_tray
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            ui_ready,
            get_app_state,
            commands::list_phones,
            commands::pair_phone,
            commands::confirm_pairing,
            commands::connect_phone,
            commands::disconnect,
            commands::forget_phone,
            commands::request_phone_start,
            commands::get_diagnostics,
            commands::subscribe_live,
            commands::set_actions_enabled,
            commands::set_settings,
            commands::start_poke_test,
            commands::stop_poke_test,
            commands::copy_diagnostics,
            commands::start_recording,
            commands::stop_recording,
            commands::mark_poke,
            commands::get_recording,
            commands::list_recordings,
            commands::delete_recording,
            commands::open_recordings_folder,
            commands::open_logs_folder
            ,commands::set_source,
            commands::set_pc_devices,
            commands::list_audio_devices
            ,commands::check_pc_sound,
            commands::cancel_pc_sound_check
        ])
        .build(tauri::generate_context!())
        .expect("Veyl could not start.")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if let Some(state) = app.try_state::<Runtime>() {
                    if !state.exiting.swap(true, Ordering::AcqRel) {
                        api.prevent_exit();
                        let _ = state.recording_events_stop.send(true);
                        let connection = state.connection.clone();
                        let recorder = state.recorder.clone();
                        let sources = state.sources.clone();
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            sources.shutdown().await;
                            connection.shutdown().await;
                            let _ = recorder.stop().await;
                            app.exit(code.unwrap_or(0));
                        });
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_default_and_typical_hotkeys_parse() {
        for hotkey in ["Ctrl+Alt+V", "Ctrl+Shift+F9", "Alt+Space"] {
            assert!(super::parse_hotkey(hotkey).is_ok(), "{hotkey}");
        }
        assert_eq!(super::parse_hotkey("Ctrl+").unwrap_err(), "invalid_hotkey");
        assert_eq!(super::tray_tooltip(false), "Veyl · Actions off");
    }
}
