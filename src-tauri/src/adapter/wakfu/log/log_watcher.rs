use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::thread;
use std::time::Duration;

use notify_debouncer_mini::notify::{self, PollWatcher, RecursiveMode};
use notify_debouncer_mini::{Config, DebounceEventResult, Debouncer, new_debouncer_opt};

use super::log_tailer::LogTailer;

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(100);

// Notify's `PollWatcher::watch` silently no-ops when the target path doesn't
// exist yet: it reads the path's metadata once to build its internal watch
// state, and if that fails it just drops the watch instead of erroring or
// retrying (see Notify's own `WatchData::new`, which says outright that a
// path missing at watch-time can never be picked up later). So even though
// `wakfu.log` is expected to already exist by the time this runs, we don't
// rely on that: `spawn_watch_when_ready` below polls for the file and only
// registers the real watch once it's actually there, so a missing log at
// startup delays detection instead of silently disabling it forever.
pub fn watch(
    log_path: &Path,
    mut on_new_lines: impl FnMut(Vec<String>) + Send + 'static,
) -> notify::Result<Arc<Mutex<Debouncer<PollWatcher>>>> {
    let mut tailer = LogTailer::new(log_path.to_path_buf());

    let notify_config = notify::Config::default().with_poll_interval(POLL_INTERVAL);
    let config = Config::default()
        .with_timeout(DEBOUNCE_TIMEOUT)
        .with_notify_config(notify_config);

    let debouncer =
        new_debouncer_opt::<_, PollWatcher>(config, move |result: DebounceEventResult| {
            if let Err(err) = result {
                log::error!("wakfu log watch error: {err:?}");
                return;
            }
            match tailer.read_new_lines() {
                Ok(lines) => on_new_lines(lines),
                Err(err) => log::error!("failed to read new wakfu log lines: {err}"),
            }
        })?;

    let debouncer = Arc::new(Mutex::new(debouncer));
    spawn_watch_when_ready(log_path.to_path_buf(), Arc::downgrade(&debouncer));

    Ok(debouncer)
}

// Waits (polling on `POLL_INTERVAL`) for `log_path` to exist, then registers
// the actual notify watch. Runs on its own thread so a missing log file
// never blocks app startup. Holds only a `Weak` ref so the thread can't keep
// the debouncer (and the app) alive forever if `log_path` never shows up.
fn spawn_watch_when_ready(log_path: PathBuf, debouncer: Weak<Mutex<Debouncer<PollWatcher>>>) {
    thread::spawn(move || {
        while !log_path.exists() {
            if debouncer.upgrade().is_none() {
                return;
            }
            thread::sleep(POLL_INTERVAL);
        }

        let Some(debouncer) = debouncer.upgrade() else {
            return;
        };

        let mut guard = match debouncer.lock() {
            Ok(guard) => guard,
            Err(err) => {
                log::error!("wakfu log watcher mutex poisoned: {err}");
                return;
            }
        };

        if let Err(err) = guard.watcher().watch(&log_path, RecursiveMode::NonRecursive) {
            log::error!("failed to start watching wakfu log file: {err:?}");
        }
    });
}
