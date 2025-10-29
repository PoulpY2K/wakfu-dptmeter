use notify::{PollWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer_opt, Config, DebounceEventResult, Debouncer};
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use chrono::{DateTime, TimeZone};
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
use serde::{de::Error, Deserialize, Deserializer};
use serde_json;

const WAKFU_CHAT_LOG_PATH: &str =
    "C:\\Users\\poulpyy\\AppData\\Roaming\\zaap\\gamesLogs\\wakfu\\logs\\wakfu.log";

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
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
        .setup(|_app| {
            thread::spawn(|| {
                // Setup debouncer
                log::info!(
                    "Setup debouncing watch for chat log file {}",
                    WAKFU_CHAT_LOG_PATH
                );
                let (tx, rx) = std::sync::mpsc::channel();
                let mut debouncer = declare_debouncer(tx);

                // Empty file on startup
                log::info!("Emptying chat log file at startup");
                if let Err(e) = File::create(WAKFU_CHAT_LOG_PATH).and_then(|f| f.set_len(0)) {
                    log::error!("Failed to empty chat log file: {}", e);
                }

                log::info!("Initializing watch and waiting for events");
                let file_size: u64 = 0;
                debouncer
                    .watcher()
                    .watch(Path::new(WAKFU_CHAT_LOG_PATH), RecursiveMode::NonRecursive)
                    .unwrap();

                // Treat results
                for result in rx {
                    match result {
                        Ok(_event) => {
                            let buf = get_recent_lines_buffer_from_file(file_size);
                            match String::from_utf8(buf) {
                                Ok(s) => {
                                    log::info!("{}", s.trim())
                                }
                                Err(_) => log::info!("Une erreur est survenue lors de la récupération du texte"),
                            }
                        }
                        Err(error) => println!("Error {error:?}"),
                    }
                }
            });

            Ok(())
        })
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

fn declare_debouncer(tx: Sender<DebounceEventResult>) -> Debouncer<PollWatcher> {
    // notify backend configuration
    let backend_config = notify::Config::default().with_poll_interval(Duration::from_millis(100));
    // debouncer configuration
    let debouncer_config = Config::default()
        .with_timeout(Duration::from_millis(100))
        .with_notify_config(backend_config);
    // select backend via fish operator, here PollWatcher backend
    new_debouncer_opt::<_, PollWatcher>(debouncer_config, tx).unwrap()
}

fn get_recent_lines_buffer_from_file(mut file_size: u64) -> Vec<u8> {
    let mut f = File::open(WAKFU_CHAT_LOG_PATH).unwrap();
    let metadata = f.metadata().unwrap();
    let new_size = metadata.len();

    // si le fichier a été tronqué ou recréé, repartir depuis 0
    let added = if new_size >= file_size {
        new_size - file_size
    } else {
        file_size = 0;
        new_size
    };

    // se positionner à l'ancienne fin et lire exactement `added` octets
    f.seek(SeekFrom::Start(file_size)).unwrap();
    let mut buf = Vec::with_capacity(added as usize);
    let mut reader = f.take(added);
    reader.read_to_end(&mut buf).unwrap();

    // mettre à jour la taille connue
    file_size = new_size;

    buf
}

#[derive(Debug)]
struct Fight {
    pub fighters: Vec<Fighter>,
    pub started: DateTime<chrono::Utc>,
    pub finished: DateTime<chrono::Utc>
}

#[derive(Debug)]
struct Fighter {
    pub id: u64,
    pub name: String,
    pub total_damage: u64,
    pub is_controlled_by_ai: bool
}