//! Reload (ticket 10): showing a window's document again after its file changed, or when the
//! reader asks (F5, Ctrl+R and the right-click menu). The page keeps its reading position; here
//! the file is watched, its new render started and the page told.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Emitter as _, Manager as _, WebviewWindow};

use crate::notice;
use crate::watch::{Change, Watch};
use crate::window::OpenWindows;

/// The event that tells a window's page what happened to its document's file: `reload` (it was
/// saved or came back, or the reader asked, and its new render has started) or `gone`.
const DOCUMENT_CHANGED: &str = "document-changed";

/// Reloads the calling window's document by hand (F5, Ctrl+R and the right-click menu), the way a
/// save to its file does: the page is told, and then asks for the new render.
#[tauri::command]
pub fn reload_document(window: WebviewWindow) -> tauri::Result<()> {
    let (app, label) = (window.app_handle(), window.label());
    if !app.state::<OpenWindows>().reload_ahead(label) {
        return Ok(());
    }
    app.emit_to(label, DOCUMENT_CHANGED, "reload")
}

/// Starts watching the file of `document`, which `window` was just given with `key`, on a blocking
/// thread, since that reads the file system. A watch that can't start leaves the window without
/// reloads on save, and the reader is told.
pub fn watch_document(app: &AppHandle, window: &str, document: &Path, key: PathBuf) {
    let (app, window, document) = (app.clone(), window.to_owned(), document.to_owned());
    tauri::async_runtime::spawn_blocking(move || {
        let on_change = {
            let (app, window, key) = (app.clone(), window.clone(), key.clone());
            move |change| on_document_change(&app, &window, &key, change)
        };
        let open_windows = app.state::<OpenWindows>();
        match Watch::start(&document, on_change) {
            Ok(watch) => open_windows.keep_watch(&window, &key, watch),
            Err(error) => {
                let message = format!(
                    "Can't watch {} for changes, so only F5 reloads it: {error}",
                    document.display()
                );
                if let Some(message) = open_windows.hold_notice(&window, &key, message) {
                    notice::show_notice(&app, &window, &message);
                }
            }
        }
    });
}

/// Acts on a change to the file of the document with `key`, unless `window` has left it since.
fn on_document_change(app: &AppHandle, window: &str, key: &Path, change: Change) {
    let open_windows = app.state::<OpenWindows>();
    // Checked and acted on under one lock (ticket 21): a document the window is given after a
    // separate check would have its render ahead replaced by one for the window's new document.
    let (still_shown, event) = match change {
        Change::Changed | Change::Back => {
            (open_windows.reload_ahead_while_shown(window, key), "reload")
        }
        Change::Gone => (open_windows.shows(window, key), "gone"),
    };
    if still_shown {
        // The only way to fail is that the window has closed, and then nothing is left to do.
        let _ = app.emit_to(window, DOCUMENT_CHANGED, event);
    }
}
