//! Keep on top (window design ticket 07): one window stays above all others. It belongs to the
//! window, not the settings: nothing saves it, and a new window starts without it, as
//! `window::open_window` builds every window with Tauri's default of off.
//!
//! Tauri's cross-platform window API switches it (on Windows, tao sets `HWND_TOPMOST`), so it
//! needs no code in the per-OS `platform` module, which holds what Tauri doesn't cover.

use tauri::WebviewWindow;

use crate::notice::Notice;
use crate::window;

/// What `switch_keep_on_top` answers: whether the window is kept on top now. Named so that the
/// contract sample is of the answer's own type.
pub type KeptOnTop = bool;

/// Keep on top's button, Ctrl+Alt+T or its menu entry: switches the calling window, and only it,
/// and answers whether it is kept on top now.
#[tauri::command]
pub fn switch_keep_on_top(window: WebviewWindow) -> Result<KeptOnTop, Notice> {
    window::switch_state(
        &window,
        WebviewWindow::is_always_on_top,
        WebviewWindow::set_always_on_top,
        "Keep on top",
    )
}

/// A sample of each answer of `switch_keep_on_top`, which the page's tests show on its button
/// (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    let (on, off): (KeptOnTop, KeptOnTop) = (true, false);
    Ok(serde_json::json!({
        "switchedOn": serde_json::to_value(on)?,
        "switchedOff": serde_json::to_value(off)?,
    }))
}
