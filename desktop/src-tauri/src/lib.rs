mod settings;

use settings::Settings;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

struct Runtime {
    settings: Settings,
    hidden: bool,
    _logging: tracing_appender::non_blocking::WorkerGuard,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppState {
    version: &'static str,
    pc_name: String,
    actions_enabled: bool,
}

#[tauri::command]
fn get_app_state(state: tauri::State<'_, Runtime>) -> AppState {
    AppState {
        version: env!("CARGO_PKG_VERSION"),
        pc_name: state.settings.pc_name.clone(),
        actions_enabled: false,
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
            let settings = Settings::load_or_create(&directory.join("settings.json"))?;
            app.manage(Runtime {
                settings,
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
            tracing::info!(
                "Veyl started. Capture and actions are unavailable in this preview build."
            );
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.state::<Runtime>().settings.close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![ui_ready, get_app_state])
        .run(tauri::generate_context!())
        .expect("Veyl could not start.");
}
