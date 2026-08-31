use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};

const RESET_FIGHT: &str = "reset_fight";
const TOGGLE_LOCK: &str = "toggle_lock";
const TOGGLE_ALWAYS_ON_TOP: &str = "toggle_always_on_top";

/// Rust-owned source of truth for "locked" — unlike always-on-top, there's
/// no real OS window property to read this back from, and round-tripping
/// through `CheckMenuItem::is_checked()` proved unreliable (it consistently
/// returned the wrong value on Windows), so we track it ourselves instead.
#[derive(Default)]
pub struct LockState(AtomicBool);

/// Snapshot of overlay UI state, for the frontend to hydrate itself on
/// startup — otherwise a webview reload shows stale defaults that can
/// disagree with Rust's actual state (e.g. after toggling via the tray
/// before the frontend has loaded/reloaded).
#[derive(Serialize)]
pub struct OverlayState {
    locked: bool,
    always_on_top: bool,
}

/// Builds the menu shared by the overlay's right-click context menu and the
/// tray icon's menu — one definition of "what the app's menu contains".
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let reset_fight = MenuItem::with_id(app, RESET_FIGHT, "Reset current fight", true, None::<&str>)?;
    let toggle_lock =
        CheckMenuItem::with_id(app, TOGGLE_LOCK, "Lock position", true, false, None::<&str>)?;
    // Starts checked: the window already boots with alwaysOnTop: true (see tauri.conf.json).
    let toggle_always_on_top = CheckMenuItem::with_id(
        app,
        TOGGLE_ALWAYS_ON_TOP,
        "Always on top",
        true,
        true,
        None::<&str>,
    )?;
    let quit = PredefinedMenuItem::quit(app, Some("Quit"))?;

    Menu::with_items(
        app,
        &[
            &reset_fight,
            &PredefinedMenuItem::separator(app)?,
            &toggle_lock,
            &toggle_always_on_top,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

/// Dispatches a click on the tray's native menu (`Quit` is handled natively
/// by `PredefinedMenuItem::quit` and never reaches here).
// `App::on_menu_event` requires `Fn(&AppHandle<R>, MenuEvent)`; `MenuEvent` can't be borrowed here.
#[allow(clippy::needless_pass_by_value)]
pub fn handle_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        RESET_FIGHT => {
            if let Err(err) = app.emit("overlay-reset-fight", ()) {
                log::error!("failed to emit overlay-reset-fight: {err}");
            }
        }
        TOGGLE_LOCK => {
            apply_toggle_lock(app);
        }
        TOGGLE_ALWAYS_ON_TOP => {
            apply_toggle_always_on_top(app);
        }
        _ => {}
    }
}

/// Toggles the lock state, updates the menu checkbox, and emits
/// `overlay-locked-changed` — callable from both the native tray menu and
/// the overlay's custom menu, so the two surfaces never disagree.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn toggle_lock(app: AppHandle) -> Result<bool, String> {
    apply_toggle_lock(&app).ok_or_else(|| "failed to toggle lock position".to_string())
}

/// Toggles the window's always-on-top property, updates the menu checkbox,
/// and emits `overlay-always-on-top-changed` — callable from both the
/// native tray menu and the overlay's custom menu.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn toggle_always_on_top(app: AppHandle) -> Result<bool, String> {
    apply_toggle_always_on_top(&app).ok_or_else(|| "failed to toggle always-on-top".to_string())
}

/// Quits the app, for the overlay's custom menu (the tray's native Quit
/// item is handled by `PredefinedMenuItem::quit` instead).
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn get_overlay_state(app: AppHandle) -> Result<OverlayState, String> {
    let lock_state = app
        .try_state::<LockState>()
        .ok_or_else(|| "lock state not initialized".to_string())?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let always_on_top = window.is_always_on_top().map_err(|err| err.to_string())?;
    Ok(OverlayState {
        locked: lock_state.0.load(Ordering::SeqCst),
        always_on_top,
    })
}

fn apply_toggle_lock(app: &AppHandle) -> Option<bool> {
    let lock_state = app.try_state::<LockState>()?;
    let menu = app.try_state::<Menu<Wry>>()?;
    let new_state = !lock_state.0.fetch_xor(true, Ordering::SeqCst);
    set_checked(&menu, TOGGLE_LOCK, new_state);
    if let Err(err) = app.emit("overlay-locked-changed", new_state) {
        log::error!("failed to emit overlay-locked-changed: {err}");
    }
    Some(new_state)
}

fn apply_toggle_always_on_top(app: &AppHandle) -> Option<bool> {
    let window = app.get_webview_window("main")?;
    let menu = app.try_state::<Menu<Wry>>()?;
    let new_state = !window.is_always_on_top().unwrap_or(true);
    if let Err(err) = window.set_always_on_top(new_state) {
        log::error!("failed to toggle always-on-top: {err}");
        return None;
    }
    set_checked(&menu, TOGGLE_ALWAYS_ON_TOP, new_state);
    if let Err(err) = app.emit("overlay-always-on-top-changed", new_state) {
        log::error!("failed to emit overlay-always-on-top-changed: {err}");
    }
    Some(new_state)
}

fn set_checked(menu: &Menu<Wry>, id: &str, checked: bool) {
    if let Some(item) = menu.get(id).and_then(|item| item.as_check_menuitem().cloned()) {
        set_checked_item(&item, id, checked);
    }
}

fn set_checked_item(item: &CheckMenuItem<Wry>, id: &str, checked: bool) {
    if let Err(err) = item.set_checked(checked) {
        log::error!("failed to update menu item '{id}' checked state: {err}");
    }
}

/// Adds a tray icon showing the shared menu on right-click. Logs and skips
/// if the app has no default window icon to use for it.
pub fn setup_tray(app: &tauri::App, menu: &Menu<Wry>) {
    let Some(icon) = app.default_window_icon().cloned() else {
        log::warn!("no default window icon; skipping tray icon setup");
        return;
    };

    if let Err(err) = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("Wakfu DPT Meter")
        .menu(menu)
        .build(app)
    {
        log::error!("failed to build tray icon: {err}");
    }
}
