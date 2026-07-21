mod path;
mod tailer;

use std::path::Path;
use std::time::Duration;

use notify_debouncer_mini::notify::{self, PollWatcher, RecursiveMode};
use notify_debouncer_mini::{Config, DebounceEventResult, Debouncer, new_debouncer_opt};
use tauri::{AppHandle, Emitter};

pub use path::wakfu_log_path;
use tailer::LogTailer;

// Windows' native file-change notifications (ReadDirectoryChangesW) are
// documented by notify itself as unreliable for files written by another
// process. PollWatcher checks file metadata on a fixed interval instead of
// relying on OS-delivered events, trading a bit of latency for events that
// actually always arrive.
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(100);

// The tool is read-only: it never creates the log file itself, and only
// installs on machines where the player already has Wakfu running (and thus
// already has a `wakfu.log`), so the file is assumed to exist by the time
// this is called.
pub fn watch_log_file(
    app_handle: AppHandle,
    log_path: &Path,
) -> notify::Result<Debouncer<PollWatcher>> {
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
        let log_event = crate::domain::log_parsing::parse_line(&line);
        if !matches!(
            log_event,
            crate::domain::log_parsing::LogEvent::Unrecognized
        ) {
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
