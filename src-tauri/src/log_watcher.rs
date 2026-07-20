use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Duration;
use notify_debouncer_mini::notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use tauri::{AppHandle, Emitter};

pub fn wakfu_log_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").expect("APPDATA environment variable is not set");
    PathBuf::from(appdata)
        .join("zaap")
        .join("gamesLogs")
        .join("wakfu")
        .join("logs")
        .join("wakfu.log")
}

pub fn watch_log_file(
    app_handle: AppHandle,
    log_path: PathBuf,
) -> notify_debouncer_mini::notify::Result<Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !log_path.exists() {
        std::fs::File::create(&log_path)?;
    }

    let mut tailer = LogTailer::new(log_path.clone())?;
    let mut tracker = crate::fight_tracker::FightTracker::new();

    let mut debouncer = new_debouncer(Duration::from_millis(500), move |result: DebounceEventResult| {
        if let Err(err) = result {
            log::error!("wakfu log watch error: {err:?}");
            return;
        }

        let lines = match tailer.read_new_lines() {
            Ok(lines) => lines,
            Err(err) => {
                log::error!("failed to read new wakfu log lines: {err}");
                return;
            }
        };

        for line in lines {
            let log_event = crate::log_parser::parse_line(&line);
            for fight_event in tracker.process(log_event) {
                if let Err(err) = app_handle.emit("fight-event", &fight_event) {
                    log::error!("failed to emit fight-event: {err}");
                }
            }
        }
    })?;

    debouncer
        .watcher()
        .watch(&log_path, RecursiveMode::NonRecursive)?;

    Ok(debouncer)
}

pub struct LogTailer {
    path: PathBuf,
    position: u64,
}

impl LogTailer {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        let position = std::fs::metadata(&path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        Ok(Self { path, position })
    }

    pub fn read_new_lines(&mut self) -> std::io::Result<Vec<String>> {
        let mut file = File::open(&self.path)?;
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
            .map(|line| line.to_string())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;

    fn temp_log_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(name);
        path
    }

    #[test]
    fn ignores_content_written_before_the_tailer_was_created() {
        let path = temp_log_path("wakfu_tailer_test_existing_content.txt");
        fs::write(&path, "LIGNE 1\nLIGNE 2\n").unwrap();

        let mut tailer = LogTailer::new(path.clone()).unwrap();
        let lines = tailer.read_new_lines().unwrap();

        assert!(lines.is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn returns_only_lines_appended_after_creation() {
        let path = temp_log_path("wakfu_tailer_test_appended_content.txt");
        fs::write(&path, "LIGNE 1\n").unwrap();

        let mut tailer = LogTailer::new(path.clone()).unwrap();

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

        let mut tailer = LogTailer::new(path.clone()).unwrap();

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
