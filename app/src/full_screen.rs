//! Full screen (full screen ticket 01): one window fills its whole screen, over the taskbar. It
//! belongs to the window, as Keep on top does, not the settings: nothing saves it, and a new
//! window starts out of it, as `window::open_window` builds every window with Tauri's default of
//! off.
//!
//! Tauri's cross-platform window API switches it, so it needs no code in the per-OS `platform`
//! module. On Windows, tauri-runtime-wry 2.12 always asks tao 0.37 for a borderless window
//! covering the window's current monitor, never an exclusive one, so other windows still go over
//! it. Leaving needs no memory of its own: tao keeps the window's placement (`GetWindowPlacement`)
//! when it goes full screen, a maximized window's included, and puts it back on leaving, so a
//! maximized window comes back maximized and any other at its size and place (the ticket's
//! comments; a hand check confirms it).

use tauri::WebviewWindow;

use crate::notice::Notice;
use crate::window;

/// What `switch_full_screen` answers: whether the window is full screen now. Named so that the
/// contract sample is of the answer's own type.
pub type FullScreen = bool;

/// F11 or Full screen's menu entry: switches the calling window, and only it, and answers whether
/// it is full screen now.
#[tauri::command]
pub fn switch_full_screen(window: WebviewWindow) -> Result<FullScreen, Notice> {
    window::switch_state(
        &window,
        WebviewWindow::is_fullscreen,
        WebviewWindow::set_fullscreen,
        "Full screen",
    )
}

/// A sample of each answer of `switch_full_screen`, which the page's tests show on its menu entry
/// (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    let (on, off): (FullScreen, FullScreen) = (true, false);
    Ok(serde_json::json!({
        "switchedOn": serde_json::to_value(on)?,
        "switchedOff": serde_json::to_value(off)?,
    }))
}
