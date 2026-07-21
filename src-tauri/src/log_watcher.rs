use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

use notify_debouncer_mini::notify::{self, PollWatcher, RecursiveMode};
use notify_debouncer_mini::{Config, DebounceEventResult, Debouncer, new_debouncer_opt};
use tauri::{AppHandle, Emitter};

/// Failure modes when resolving the Wakfu client's log file path.
///
/// Each variant is only ever constructed on the platform it targets, hence
/// the matching `#[cfg]` gates: keeping the enum shape aligned with
/// [`wakfu_log_path`]'s own per-OS branches avoids dead-code warnings when
/// a single-platform build only compiles one branch.
#[derive(Debug, thiserror::Error)]
pub enum LogPathError {
    #[cfg(target_os = "windows")]
    #[error("APPDATA environment variable is not set")]
    AppDataNotSet,
    #[cfg(target_os = "macos")]
    #[error("HOME environment variable is not set")]
    HomeNotSet,
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    #[error("unsupported OS for wakfu log path")]
    UnsupportedOs,
}

pub fn wakfu_log_path() -> Result<PathBuf, LogPathError> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").map_err(|_| LogPathError::AppDataNotSet)?;
        Ok(PathBuf::from(appdata)
            .join("zaap")
            .join("gamesLogs")
            .join("wakfu")
            .join("logs")
            .join("wakfu.log"))
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| LogPathError::HomeNotSet)?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join("zaap")
            .join("wakfu")
            .join("logs")
            .join("wakfu.log"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err(LogPathError::UnsupportedOs)
    }
}

// Windows' native file-change notifications (ReadDirectoryChangesW) are
// documented by notify itself as unreliable for files written by another
// process. PollWatcher checks file metadata on a fixed interval instead of
// relying on OS-delivered events, trading a bit of latency for events that
// actually always arrive.
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(100);

pub fn watch_log_file(
    app_handle: AppHandle,
    log_path: &Path,
) -> notify::Result<Debouncer<PollWatcher>> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !log_path.exists() {
        File::create(log_path)?;
    }

    let mut tailer = LogTailer::new(log_path.to_path_buf());
    let mut tracker = crate::fight_tracker::FightTracker::new();

    let notify_config = notify::Config::default().with_poll_interval(POLL_INTERVAL);
    let config = Config::default()
        .with_timeout(DEBOUNCE_TIMEOUT)
        .with_notify_config(notify_config);

    let mut debouncer =
        new_debouncer_opt::<_, PollWatcher>(config, move |result: DebounceEventResult| {
            if let Err(err) = result {
                log::error!("wakfu log watch error: {err:?}");
                return;
            }
            process_new_lines(&mut tailer, &mut tracker, &app_handle);
        })?;

    debouncer
        .watcher()
        .watch(log_path, RecursiveMode::NonRecursive)?;

    Ok(debouncer)
}

fn process_new_lines(
    tailer: &mut LogTailer,
    tracker: &mut crate::fight_tracker::FightTracker,
    app_handle: &AppHandle,
) {
    let lines = match tailer.read_new_lines() {
        Ok(lines) => lines,
        Err(err) => {
            log::error!("failed to read new wakfu log lines: {err}");
            return;
        }
    };

    for line in lines {
        let log_event = crate::log_parser::parse_line(&line);
        if !matches!(log_event, crate::log_parser::LogEvent::Unrecognized) {
            log::debug!("wakfu log parsed: {log_event:?}");
        }
        for fight_event in tracker.process(log_event) {
            log::debug!("fight-event emitted: {fight_event:?}");
            if let Err(err) = app_handle.emit("fight-event", &fight_event) {
                log::error!("failed to emit fight-event: {err}");
            }
        }
    }
}

pub struct LogTailer {
    path: PathBuf,
    position: u64,
}

impl LogTailer {
    pub fn new(path: PathBuf) -> Self {
        let position = std::fs::metadata(&path).map_or(0, |metadata| metadata.len());
        Self { path, position }
    }

    pub fn read_new_lines(&mut self) -> std::io::Result<Vec<String>> {
        let mut file = File::open(&self.path)?;
        let len = file.metadata()?.len();
        if len < self.position {
            self.position = len;
        }
        file.seek(SeekFrom::Start(self.position))?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        if buf.is_empty() {
            return Ok(Vec::new());
        }

        let Some(last_newline) = buf.iter().rposition(|&byte| byte == b'\n') else {
            return Ok(Vec::new());
        };

        let complete = &buf[..=last_newline];
        self.position += complete.len() as u64;

        Ok(String::from_utf8_lossy(complete)
            .lines()
            .map(ToString::to_string)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    #[cfg(target_os = "macos")]
    use std::sync::{Mutex, OnceLock};

    use super::*;

    fn temp_log_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(name);
        path
    }

    #[cfg(target_os = "macos")]
    fn home_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn resolves_log_path_under_home_library_logs() {
        let _guard = home_env_lock().lock().unwrap();
        let previous = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", "/tmp/wakfu_dptmeter_fake_home");
        }

        let path = wakfu_log_path().unwrap();

        unsafe {
            match &previous {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert_eq!(
            path,
            PathBuf::from("/tmp/wakfu_dptmeter_fake_home/Library/Logs/zaap/wakfu/logs/wakfu.log")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn errors_when_home_is_not_set() {
        let _guard = home_env_lock().lock().unwrap();
        let previous = std::env::var("HOME").ok();
        unsafe {
            std::env::remove_var("HOME");
        }

        let result = wakfu_log_path();

        unsafe {
            if let Some(value) = &previous {
                std::env::set_var("HOME", value);
            }
        }

        assert!(result.is_err());
    }

    #[test]
    fn ignores_content_written_before_the_tailer_was_created() {
        let path = temp_log_path("wakfu_tailer_test_existing_content.txt");
        fs::write(&path, "LIGNE 1\nLIGNE 2\n").unwrap();

        let mut tailer = LogTailer::new(path.clone());
        let lines = tailer.read_new_lines().unwrap();

        assert!(lines.is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn returns_only_lines_appended_after_creation() {
        let path = temp_log_path("wakfu_tailer_test_appended_content.txt");
        fs::write(&path, "LIGNE 1\n").unwrap();

        let mut tailer = LogTailer::new(path.clone());

        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "CREATION DU COMBAT").unwrap();
        writeln!(file, "LIGNE 3").unwrap();
        file.flush().unwrap();

        let lines = tailer.read_new_lines().unwrap();

        assert_eq!(
            lines,
            vec!["CREATION DU COMBAT".to_string(), "LIGNE 3".to_string()]
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn holds_back_a_trailing_line_that_has_no_terminating_newline_yet() {
        let path = temp_log_path("wakfu_tailer_test_partial_line.txt");
        fs::write(&path, "").unwrap();

        let mut tailer = LogTailer::new(path.clone());

        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        write!(file, "CREATION DU COMBAT\nLIGNE INCOMPLETE").unwrap();
        file.flush().unwrap();

        let lines = tailer.read_new_lines().unwrap();
        assert_eq!(lines, vec!["CREATION DU COMBAT".to_string()]);

        writeln!(file, " - suite").unwrap();
        file.flush().unwrap();

        let lines = tailer.read_new_lines().unwrap();
        assert_eq!(lines, vec!["LIGNE INCOMPLETE - suite".to_string()]);

        let _ = fs::remove_file(&path);
    }
}
