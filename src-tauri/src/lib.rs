mod log_parser;
mod fight_tracker;
mod log_watcher;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .setup(|app| {
            let app_handle = app.handle().clone();
            let log_path = log_watcher::wakfu_log_path();

            log::info!("Watching wakfu log file at {}", log_path.display());

            let debouncer = log_watcher::watch_log_file(app_handle, log_path)
                .expect("failed to start watching the wakfu log file");

            // Intentionally leaked: the watcher must run for the whole app
            // process, and `Debouncer` stops watching as soon as it is dropped.
            std::mem::forget(debouncer);

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
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
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
