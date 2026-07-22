use std::path::Path;
use std::sync::{Arc, Mutex};

use notify_debouncer_mini::Debouncer;
use notify_debouncer_mini::notify::{self, PollWatcher};
use tauri::AppHandle;

use crate::domain::fight::FightTracker;
use crate::domain::parser::{self, LogEvent};
use crate::adapter::{event, wakfu};

pub fn start_watching(
    app_handle: AppHandle,
    log_path: &Path,
) -> notify::Result<Arc<Mutex<Debouncer<PollWatcher>>>> {
    let mut tracker = FightTracker::new();

    wakfu::log::watch(log_path, move |lines| {
        for line in lines {
            let log_event = parser::parse_line(&line);
            if !matches!(log_event, LogEvent::Unrecognized) {
                log::debug!("{log_event:?}");
            }
            for fight_event in tracker.process(log_event) {
                log::debug!("{fight_event:?}");
                event::emit_fight(&app_handle, &fight_event);
            }
        }
    })
}
