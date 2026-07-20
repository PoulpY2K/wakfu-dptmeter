mod log_parser;
mod fight_tracker;
mod log_watcher;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

const WAKFU_CHAT_LOG_PATH: &str =
    "%APPDATA%\\zaap\\gamesLogs\\wakfu\\logs\\wakfu.log";

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

            log::info!(
                "Create threads and setup debouncing watch for log file {}",
                WAKFU_CHAT_LOG_PATH
            );

            //let (tx, rx) = std::sync::mpsc::channel();
            //let mut debouncer = declare_debouncer(tx);

            tauri::async_runtime::spawn(async move {
                log::info!("Initializing watch and waiting for events");
                /*debouncer
                    .watcher()
                    .watch(Path::new(WAKFU_CHAT_LOG_PATH), RecursiveMode::NonRecursive)
                    .unwrap();

                handle_debouncer_result(&app_handle, rx);*/
            });

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
