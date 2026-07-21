use std::path::Path;

use notify_debouncer_mini::Debouncer;
use notify_debouncer_mini::notify::{self, PollWatcher};
use tauri::AppHandle;

use crate::domain::fight::FightTracker;
use crate::domain::log_parsing::{self, LogEvent};
use crate::infra::{fight_event_emitter, wakfu_log};

pub fn start_watching(
    app_handle: AppHandle,
    log_path: &Path,
) -> notify::Result<Debouncer<PollWatcher>> {
    let mut tracker = FightTracker::new();

    wakfu_log::watch(log_path, move |lines| {
        for line in lines {
            let log_event = log_parsing::parse_line(&line);
            if !matches!(log_event, LogEvent::Unrecognized) {
                log::debug!("wakfu log parsed: {log_event:?}");
            }
            for fight_event in tracker.process(log_event) {
                log::debug!("fight-event emitted: {fight_event:?}");
                fight_event_emitter::emit(&app_handle, &fight_event);
            }
        }
    })
}
