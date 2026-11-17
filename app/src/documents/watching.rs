//! Watching the file of the document a window shows (ticket 10): a change reloads it, a file that
//! is gone tells the page, and a folder whose events can't be watched becomes a notice, held
//! until the page first asks (ticket 30). Kept apart from the table, since every report here runs
//! on a watch's thread and must check, under the table's lock, that the window still shows the
//! document it was started for (ticket 21).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

use super::{Documents, Inner};
use crate::message::Message;
use crate::notice::{NamedFile, Notice};
use crate::watch::{Change, Report, Watch};

impl Documents {
    /// Starts watching `document`'s file, which `window` was just given with `key`, from the
    /// stamp the render ahead read it at (ticket 30). Started under the lock that finds the
    /// window still showing the document, so a window that has left it since starts none, and a
    /// watch never outlives the document it was started for.
    pub(super) fn watch(&self, window: &str, document: &Path, key: PathBuf) {
        let on_report = Self::on_report(
            Arc::downgrade(&self.inner),
            window.to_owned(),
            document.to_owned(),
            key.clone(),
        );
        let mut table = self.lock();
        if let Some(shown) = table.shown_mut(window, &key)
            && let Some(read_stamp) = shown.read_stamp.take()
        {
            shown.watch = Some(Watch::start(document, read_stamp, on_report));
        }
    }

    /// What the watch on `document`, which `window` was given with `key`, does with each report:
    /// a change reloads the document or tells the page it is gone, and a folder whose events
    /// can't be watched tells the reader that saves reload late.
    pub(super) fn on_report(
        // Weak, so that a watch never keeps the table that holds it alive.
        documents: Weak<Inner>,
        window: String,
        document: PathBuf,
        key: PathBuf,
    ) -> impl FnMut(Report) + Send + 'static {
        move |report| {
            let Some(documents) = documents.upgrade().map(|inner| Self { inner }) else {
                return;
            };
            match report {
                Report::Change(change) => documents.changed(&window, &key, change),
                Report::EventsUnavailable {
                    reason,
                    checked_every,
                } => {
                    let late = format!(
                        " for changes as they are made, so a save reloads it up to {} seconds \
                         later: {reason}",
                        checked_every.as_secs()
                    );
                    let notice = Notice::naming("Can't watch ", NamedFile::at(&document), late);
                    documents.notice(&window, &key, notice);
                }
            }
        }
    }

    /// Acts on a change to the file of the document with `key`, unless `window` has left it since
    /// or closed: a watch can report a change after that, and a document the window has left is
    /// never reloaded. Checked and acted on under one lock (ticket 21): a document the window is
    /// given after a separate check would have its render ahead replaced by one for the window's
    /// new document. The page is told after the lock, never under it, so the message can still
    /// reach it after the one for a document given in between; the page then asks with the
    /// showing its view still shows, and is answered a show (ticket 23), or, for a `gone`, sees
    /// that it names another showing than its view's (ticket 26).
    fn changed(&self, window: &str, key: &Path, change: Change) {
        let message = {
            let mut table = self.lock();
            let Some(shown) = table.shown_mut(window, key) else {
                return;
            };
            match change {
                Change::Changed | Change::Back => {
                    shown.start_reload();
                    Message::AskForDocument
                }
                Change::Gone => Message::Gone(shown.showing),
            }
        };
        // Sent from the watch's thread, which nobody waits on, so nobody is left to tell.
        let _ = self.send(window, message);
    }

    /// Sends `window`'s page the `notice` about the document with `key`, or holds it until the
    /// page first asks for that document, since until then the page may not be listening. A
    /// notice for a document the window has left, or for a closed window, is dropped.
    pub(super) fn notice(&self, window: &str, key: &Path, notice: Notice) {
        {
            let mut table = self.lock();
            let Some(shown) = table.shown_mut(window, key) else {
                return;
            };
            if !shown.page_asked {
                shown.held_notice = Some(notice);
                return;
            }
        }
        // Sent from the watch's thread, which nobody waits on, so nobody is left to tell.
        let _ = self.send(window, Message::Notice(notice));
    }
}
