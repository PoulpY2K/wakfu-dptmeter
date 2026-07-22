//! Tauri backend for the Wakfu DPT-meter: tails the Wakfu client's log file,
//! parses combat events, tracks per-fight damage/heal attribution, and
//! forwards `fight-event` payloads to the webview frontend.

mod domain;
mod adapter;
mod application;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

#[cfg_attr(not(debug_assertions), allow(unused_variables))]
fn open_devtools_if_debug(app: &tauri::App) {
    #[cfg(debug_assertions)]
    {
        match app.get_webview_window("main") {
            Some(window) => window.open_devtools(),
            None => log::warn!("main window not found; skipping devtools"),
        }
    }
}

fn start_log_watcher(app: &tauri::App) {
    let app_handle = app.handle().clone();
    let log_path = match adapter::wakfu::log::get_path() {
        Ok(path) => path,
        Err(err) => {
            log::error!("failed to resolve the wakfu log file path: {err}");
            return;
        }
    };

    log::info!("Watching wakfu log file at {}", log_path.display());

    match application::start_watching(app_handle, &log_path) {
        Ok(debouncer) => {
            app.manage(debouncer);
        }
        Err(err) => {
            log::error!("failed to start watching the wakfu log file: {err}");
        }
    }
}

/// Boots the Tauri application: registers plugins, starts the wakfu log
/// watcher, and blocks until the app window closes.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            open_devtools_if_debug(app);
            start_log_watcher(app);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                .target(Target::new(TargetKind::Webview))
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{} {} {}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                        record.level(),
                        record.target(),
                        message
                    ));
                })
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
