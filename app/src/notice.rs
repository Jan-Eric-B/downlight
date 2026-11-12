//! Notices: short messages a window's page shows the reader for a few seconds, such as why a
//! dropped file didn't open or why its document's file isn't watched.

use tauri::{AppHandle, Emitter as _};

/// The event that gives a window's page a short message to show the reader.
const NOTICE: &str = "notice";

/// Gives `window`'s page a short message to show the reader.
pub fn show_notice(app: &AppHandle, window: &str, message: &str) {
    // The only way to fail is that the window has closed, and then nobody is left to tell.
    let _ = app.emit_to(window, NOTICE, message);
}
