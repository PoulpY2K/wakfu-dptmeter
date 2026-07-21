use tauri::{AppHandle, Emitter};

use crate::domain::fight::FightEvent;

pub fn emit(app_handle: &AppHandle, event: &FightEvent) {
    if let Err(err) = app_handle.emit("fight-event", event) {
        log::error!("failed to emit fight-event: {err}");
    }
}
