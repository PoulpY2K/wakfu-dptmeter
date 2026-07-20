mod log_parser;
mod fight_tracker;
mod log_watcher;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

#[cfg_attr(not(debug_assertions), allow(unused_variables))]
fn open_devtools_if_debug(app: &tauri::App) {
    #[cfg(debug_assertions)]
    {
        let window = app.get_webview_window("main").unwrap();
        window.open_devtools();
    }
}

fn start_log_watcher(app: &tauri::App) {
    let app_handle = app.handle().clone();
    let log_path = log_watcher::wakfu_log_path();

    log::info!("Watching wakfu log file at {}", log_path.display());

    match log_watcher::watch_log_file(app_handle, log_path) {
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
                .level(log::LevelFilter::Debug)
                .target(Target::new(TargetKind::Webview))
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{} {} {}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                        record.level(),
                        record.target(),
                        message
                    ))
                })
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
