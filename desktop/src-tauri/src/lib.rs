pub mod address;
mod commands;
pub mod connection;
pub mod discovery;
pub mod engine_host;
pub mod events;
pub mod pairings;
mod settings;
pub mod source;

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
}

#[tauri::command]
fn get_app_state(state: tauri::State<'_, Runtime>) -> AppState {
    AppState {
        version: env!("CARGO_PKG_VERSION"),
        pc_name: state.settings.lock().unwrap().pc_name.clone(),
        actions_enabled: false,
        link: state.connection.snapshot(),
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
            )?;
            app.manage(Runtime {
                settings,
                connection,
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
            commands::copy_diagnostics
        ])
        .build(tauri::generate_context!())
        .expect("Veyl could not start.")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if let Some(state) = app.try_state::<Runtime>() {
                    if !state.exiting.swap(true, Ordering::AcqRel) {
                        api.prevent_exit();
                        let connection = state.connection.clone();
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            connection.shutdown().await;
                            app.exit(code.unwrap_or(0));
                        });
                    }
                }
            }
        });
}
