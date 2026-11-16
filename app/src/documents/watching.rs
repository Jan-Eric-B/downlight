//! Watching the file of the document a window shows (ticket 10): a change reloads it, a file that
//! is gone tells the page, and a watch that can't start becomes a notice, held until the page
//! first asks. Kept apart from the table, since every callback here runs on a watch's thread and
//! must check, under the table's lock, that the window still shows the document it was started
//! for (ticket 21).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

use super::{Documents, Inner};
use crate::message::Message;
use crate::notice::{NamedFile, Notice};
use crate::watch::{Change, Watch};

impl Documents {
    /// Starts watching `document`'s file, which `window` was just given with `key`, on a blocking
    /// thread, since that reads the file system. A watch that can't start leaves the window
    /// without reloads on save, and the reader is told.
    pub(super) fn watch(&self, window: &str, document: &Path, key: PathBuf) {
        // Weak, so that a watch never keeps the table that holds it alive.
        let documents = Arc::downgrade(&self.inner);
        let (window, document) = (window.to_owned(), document.to_owned());
        tauri::async_runtime::spawn_blocking(move || {
            let on_change = Self::on_change(documents.clone(), window.clone(), key.clone());
            let started = Watch::start(&document, on_change);
            let Some(documents) = Self::upgrade(&documents) else {
                return;
            };
            match started {
                Ok(watch) => documents.keep_watch(&window, &key, watch),
                Err(error) => documents.notice(
                    &window,
                    &key,
                    Notice::naming(
                        "Can't watch ",
                        NamedFile::at(&document),
                        format!(" for changes, so only F5 reloads it: {error}"),
                    ),
                ),
            }
        });
    }

    /// What the watch on the document `window` was given with `key` does on a change to its file.
    pub(super) fn on_change(
        documents: Weak<Inner>,
        window: String,
        key: PathBuf,
    ) -> impl FnMut(Change) + Send + 'static {
        move |change| {
            if let Some(documents) = Self::upgrade(&documents) {
                documents.changed(&window, &key, change);
            }
        }
    }

    fn upgrade(documents: &Weak<Inner>) -> Option<Self> {
        documents.upgrade().map(|inner| Self { inner })
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

    /// Gives `window` the watch started for the document with `key`. A watch that started after
    /// the window was given another document, or closed, is dropped instead, so a save to a
    /// document the window has left never reloads it.
    pub(super) fn keep_watch(&self, window: &str, key: &Path, watch: Watch) {
        if let Some(shown) = self.lock().shown_mut(window, key) {
            shown.watch = Some(watch);
        }
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
