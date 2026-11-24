use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use tauri::{AppHandle, Emitter, Manager, Runtime, Window};

use crate::state::AppState;

const MAIN_WINDOW: &str = "main";
const ANIMATION_EVENT: &str = "background-animation";
const ANIMATION_UNSENT: u8 = 2;

static PLAYER_OPEN: AtomicBool = AtomicBool::new(false);
static LAST_ANIMATION: AtomicU8 = AtomicU8::new(ANIMATION_UNSENT);

/// The animated background runs only when it can be seen and no game needs
/// the GPU. WebView2 doesn't report a hidden window, so this is tracked here.
fn should_animate(visible: bool, minimized: bool, player_open: bool) -> bool {
    visible && !minimized && !player_open
}

pub fn animation_allowed<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.get_webview_window(MAIN_WINDOW).is_some_and(|window| {
        should_animate(
            window.is_visible().unwrap_or(false),
            window.is_minimized().unwrap_or(true),
            PLAYER_OPEN.load(Ordering::SeqCst),
        )
    })
}

/// Tells the frontend when the answer changes.
pub fn sync_animation<R: Runtime>(app: &AppHandle<R>) {
    let allowed = animation_allowed(app);
    if LAST_ANIMATION.swap(u8::from(allowed), Ordering::SeqCst) != u8::from(allowed) {
        if let Err(error) = app.emit(ANIMATION_EVENT, allowed) {
            log::warn!("could not update background animation: {error}");
        }
    }
}

pub fn set_player_open(app: &AppHandle, open: bool) {
    PLAYER_OPEN.store(open, Ordering::SeqCst);
    sync_animation(app);
}

/// Shows the window once per launch, honoring "start minimized". The window
/// starts hidden so people never see an unstyled frame.
pub fn reveal_on_launch(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if state.revealed.swap(true, Ordering::SeqCst) {
        return;
    }
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };

    let general = state.settings.get().general;
    let result = match (general.start_minimized, general.minimize_to_tray) {
        (true, true) => Ok(()),
        (true, false) => window.show().and_then(|()| window.minimize()),
        (false, _) => window.show().and_then(|()| window.set_focus()),
    };
    if let Err(error) = result {
        log::warn!("could not show window: {error}");
    }
    sync_animation(app);
}

pub fn focus_main<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let result = window
        .show()
        .and_then(|()| window.unminimize())
        .and_then(|()| window.set_focus());
    if let Err(error) = result {
        log::warn!("could not focus window: {error}");
    }
    sync_animation(app);
}

/// Hides the window for a Roblox session. Returns true only if it was on
/// screen, so a window the user minimized or sent to the tray stays that way.
pub fn hide_for_player(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return false;
    };
    let on_screen = window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false);
    if !on_screen {
        return false;
    }
    let hidden = match window.hide() {
        Ok(()) => true,
        Err(error) => {
            log::warn!("could not hide window for Roblox: {error}");
            false
        }
    };
    sync_animation(app);
    hidden
}

/// Brings the window back, unless the user already reopened it during play.
pub fn restore_after_player(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if !window.is_visible().unwrap_or(true) {
        focus_main(app);
    }
}

pub fn should_hide_on_close<R: Runtime>(window: &Window<R>) -> bool {
    window.label() == MAIN_WINDOW
        && window
            .try_state::<AppState>()
            .is_some_and(|state| state.settings.get().general.minimize_to_tray)
}

/// Reports a startup failure when no webview is available to display it.
#[cfg(windows)]
pub fn show_fatal_error(message: &str) {
    use std::ffi::c_void;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }
    const MB_ICONERROR: u32 = 0x10;

    let wide = |value: &str| value.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let text = wide(message);
    let caption = wide("BloxClient couldn't start");
    // SAFETY: both buffers are NUL-terminated and outlive this blocking call.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), MB_ICONERROR);
    }
}

#[cfg(not(windows))]
pub fn show_fatal_error(message: &str) {
    eprintln!("{message}");
}

#[cfg(test)]
mod tests {
    use super::should_animate;

    #[test]
    fn animates_only_when_seen_and_roblox_is_closed() {
        assert!(should_animate(true, false, false));
        assert!(!should_animate(false, false, false), "hidden");
        assert!(!should_animate(true, true, false), "minimized");
        assert!(!should_animate(true, false, true), "Roblox open");
    }
}
