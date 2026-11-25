//! A document on its way (ticket 38): until the page says it placed the window's document, the
//! window keeps what it was before (`WindowRecord::before_arrival`), since its page's view still
//! shows that. Stop puts it back ([`stop_document`]), and the page's word that it placed the
//! answer for the document, its render or why it can't be shown, lets it go
//! ([`document_arrived`]), so a document joins the window's history only once the reader has
//! seen it.

use tauri::{State, WebviewWindow};

use super::{Arrival, Documents, Showing};
use crate::message::Message;
use crate::timing::Timing;
use crate::watch::Change;

/// Whether Stop put the window back ([`stop_document`]): the page asks for its document again
/// only then, to show the empty window.
pub type Stopped = bool;

impl Documents {
    /// `window`'s page placed the answer for `showing`, a show, a return or a reload, its render
    /// or why it can't be shown: what the window held while that document was on its way goes,
    /// and with it the held document's watch, and a reload is no longer unplaced. What goes
    /// includes the documents Forward went to, when the arrival went somewhere new after Back,
    /// and each pasted file among them that no window holds goes too. A page whose view shows a
    /// showing the window has left since changes nothing of that. Either way Copy as Markdown
    /// copies from the bytes of the render placed now, which the page's view shows.
    pub fn arrived(&self, window: &str, showing: Showing) {
        // Dropped after the lock, since dropping a watch stops it.
        let left = {
            let mut table = self.lock();
            let Some(record) = table.windows.get_mut(window) else {
                return;
            };
            // Whatever the window shows now, the page's view shows this answer.
            record.placed(showing);
            let Some(shown) = record
                .now
                .shown
                .as_mut()
                .filter(|shown| shown.showing == showing)
            else {
                return;
            };
            shown.reload_unplaced = false;
            record.before_arrival.take()
        };
        self.let_go(left);
    }

    /// The reader stopped `window`'s document on its way, from a page whose last placed answer
    /// was for `page_shows` (None for the empty window): the window is what it was before it was
    /// given the document, retitled after it, and the document on its way is dropped, which
    /// leaves its render, and deletes its file if that is a pasted file no window holds. A change
    /// to the document put back while it was held reloads it, or tells the page it is gone.
    ///
    /// The page's view can show another document than the one held: one the window moved on from
    /// before the page placed it, as when a drop comes while the page lays out a large document.
    /// The page then asks for the one put back, which it returns to as Back returns to a document.
    ///
    /// Changes nothing, and gives false, once the document has arrived, or when the page's last
    /// answer is another showing than the one it placed last.
    pub fn stop(&self, window: &str, page_shows: Option<Showing>) -> Stopped {
        let (left, message) = {
            let mut table = self.lock();
            let Some(record) = table.windows.get_mut(window) else {
                return false;
            };
            let Some(held) = &record.before_arrival else {
                return false;
            };
            if page_shows != record.placed_showing() {
                return false;
            }
            let in_view = held.shown.as_ref().map(|shown| shown.showing) == page_shows;
            // The document on its way goes, and with it its render and its bytes.
            let left = record.leave_the_arriving();
            let put_back = record.now.shown.as_mut();
            let message = if in_view {
                put_back.and_then(|shown| {
                    let missed = shown.missed.take()?;
                    if missed != Change::Gone {
                        shown.start_reload();
                    }
                    Some(match missed {
                        Change::Gone => Message::Gone(shown.showing),
                        Change::Changed | Change::Back => Message::AskForDocument,
                    })
                })
            } else {
                // The page reads the document put back anew, so a change it missed is no matter.
                if let Some(shown) = put_back {
                    shown.missed = None;
                    shown.arrival = Arrival::Stepped;
                }
                Some(Message::AskForDocument)
            };
            (left, message)
        };
        self.let_go(left);
        // The window keeps what it was put back to: a title or a message that can't reach it is
        // nobody's to tell, as the window is closing.
        let _ = self.retitle(window);
        if let Some(message) = message {
            let _ = self.send(window, message);
        }
        true
    }
}

/// The calling window's page placed the answer for `showing` (ticket 38), its render or why it
/// can't be shown. Passes only the showing, so a page whose view shows a document the window has
/// left changes nothing.
#[tauri::command]
pub fn document_arrived(window: WebviewWindow, showing: Showing, documents: State<'_, Documents>) {
    documents.arrived(window.label(), showing);
}

/// The reader stopped the calling window's document on its way, from its wait note or with Esc
/// (ticket 38), while the page's last placed answer was for `showing`, or for no document.
/// Passes only the showing: the document the window goes back to comes from the app's own state.
/// Answers whether it stopped.
#[tauri::command]
pub fn stop_document(
    window: WebviewWindow,
    showing: Option<Showing>,
    documents: State<'_, Documents>,
    timing: State<'_, Timing>,
) -> Stopped {
    let stopped = documents.stop(window.label(), showing);
    if stopped {
        timing.stopped(window.label());
    }
    stopped
}

/// A sample of each answer [`stop_document`] gives, by name, which the page's tests read
/// (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    let (stopped, not_stopped): (Stopped, Stopped) = (true, false);
    Ok(serde_json::json!({
        "stopped": serde_json::to_value(stopped)?,
        "notStopped": serde_json::to_value(not_stopped)?,
    }))
}
