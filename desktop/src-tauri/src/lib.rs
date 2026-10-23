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
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

struct Runtime {
    settings: Arc<Mutex<Settings>>,
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
}

#[tauri::command]
fn get_app_state(state: tauri::State<'_, Runtime>) -> AppState {
    AppState {
        version: env!("CARGO_PKG_VERSION"),
        pc_name: state.settings.lock().unwrap().pc_name.clone(),
        actions_enabled: false,
        link: state.connection.snapshot(),
        source: state.sources.snapshot(),
    }
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
        .setup(|app| {
            let directory = if cfg!(debug_assertions) {
                std::env::var_os("VEYL_DATA_DIR").map(std::path::PathBuf::from)
            } else {
                None
            }
            .unwrap_or(app.path().app_local_data_dir()?);
            std::fs::create_dir_all(&directory)?;
            let log = tracing_appender::rolling::Builder::new()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("veyl")
                .filename_suffix("log")
                .max_log_files(7)
                .build(directory.join("logs"))?;
            let (writer, guard) = tracing_appender::non_blocking(log);
            tracing_subscriber::fmt()
                .with_ansi(false)
                .with_max_level(tracing::Level::INFO)
                .with_writer(writer)
                .try_init()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let settings = Arc::new(Mutex::new(Settings::load_or_create(
                &directory.join("settings.json"),
            )?));
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
                }),
                recorder.clone(),
            )?;
            let source_events = app.handle().clone();
            let sources = sources::Sources::start(settings.clone(), &directory, connection.clone(), pc_devices::DeviceMonitor::start()?, Arc::new(move |source| { let _ = source_events.emit("veyl://source", source); }));
            app.manage(Runtime {
                settings,
                connection,
                recorder,
                sources,
                recording_events_stop,
                exiting: AtomicBool::new(false),
                hidden: std::env::args().any(|arg| arg == "--hidden"),
                _logging: guard,
            });
            let open = MenuItem::with_id(app, "open", "Open Veyl", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Veyl", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let icon = app
                .default_window_icon()
                .ok_or("Veyl's icon is missing.")?
                .clone();
            TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("Veyl · Paused")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_window(app),
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
            tracing::info!("Veyl started with actions off.");
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
            commands::copy_diagnostics,
            commands::start_recording,
            commands::stop_recording,
            commands::mark_poke,
            commands::get_recording,
            commands::list_recordings,
            commands::delete_recording,
            commands::open_recordings_folder
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
