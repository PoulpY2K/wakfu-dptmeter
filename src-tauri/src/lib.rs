//! Tauri backend for the Wakfu DPT-meter: tails the Wakfu client's log file,
//! parses combat events, tracks per-fight damage/heal attribution, and
//! forwards `fight-event` payloads to the webview frontend.

mod fight_tracker;
mod log_parser;
mod log_watcher;

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
    let log_path = match log_watcher::wakfu_log_path() {
        Ok(path) => path,
        Err(err) => {
            log::error!("failed to resolve the wakfu log file path: {err}");
            return;
        }
    };

    log::info!("Watching wakfu log file at {}", log_path.display());

    match log_watcher::watch_log_file(app_handle, &log_path) {
        Ok(debouncer) => {
            // Intentionally leaked: the watcher must run for the whole app
            // process, and `Debouncer` stops watching as soon as it is dropped.
            std::mem::forget(debouncer);
        }
        Err(err) => {
            log::error!("failed to start watching the wakfu log file: {err}");
        }
    }
}

/// Boots the Tauri application: registers plugins, starts the wakfu log
/// watcher, and blocks until the app window closes.
///
/// # Panics
/// Panics if the underlying Tauri runtime fails to start.
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
