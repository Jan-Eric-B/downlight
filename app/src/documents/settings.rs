//! Changing the reader's settings and sending them to every page (window design ticket 12): a
//! change reaches every page that hears it, and is held for a page that doesn't yet, until it
//! first asks for its document (ticket 16's review). A change of the window look (Theme and
//! Backdrop, window design ticket 08) reaches every window through the [`Windows`] port too. Kept
//! apart from the table, which only records which pages hear the settings yet (its `not_hearing`).

use super::Documents;
#[cfg(doc)]
use super::Windows;
use crate::message::Message;
use crate::settings::{PageSettings, Settings, SettingsError, WindowLook};

impl Documents {
    /// Changes the settings with `change` and saves them, and sends every window's page the
    /// settings it shows if they changed (window design ticket 12), and gives every window the
    /// window look if that changed (window design ticket 08), the one way a setting changes. The
    /// pages and windows are told while the settings are still held, so that of two changes at
    /// once, they are told last what the settings hold last. A change that can't be saved holds
    /// until Downlight closes all the same, and the error says why it wasn't saved.
    pub fn change_settings(&self, change: impl FnOnce(&mut Settings)) -> Result<(), SettingsError> {
        self.inner
            .settings
            .update(change, |updated| {
                let look = updated.settings.window_look();
                if updated.before.window_look() != look {
                    self.set_every_window_s_look(look);
                }
                let shown = updated.settings.page();
                if updated.before.page() != shown {
                    self.send_settings_to_every_page(shown);
                }
            })
            .saved
    }

    /// Sends every page that hears the settings `settings`, in the order of their labels, and
    /// keeps that the others missed them, for when they first ask
    /// ([`Documents::page_hears_settings`]).
    fn send_settings_to_every_page(&self, settings: PageSettings) {
        let mut hearing = {
            let mut table = self.lock();
            let table = &mut *table;
            let mut hearing = Vec::new();
            for window in table.windows.keys() {
                match table.not_hearing.get_mut(window) {
                    Some(missed) => *missed = true,
                    None => hearing.push(window.clone()),
                }
            }
            hearing
        };
        hearing.sort();
        for window in hearing {
            // A page that can't be told is one whose webview failed to run an event, which the
            // window that made the change can't mend. What is sent to every window is a whole
            // snapshot, so that page catches up with the next one.
            let _ = self.send(&window, Message::Settings(settings));
        }
    }

    /// Gives every open window `look`, in the order of their labels. Outside the table's lock,
    /// as every call to the [`Windows`] port is. A window still being built isn't there for the
    /// adapter to change yet; its page hasn't asked for its document either, so the look is set
    /// again when it does ([`Documents::page_hears_settings`]): the look is part of what the
    /// page shows, so the change marked the page as having missed it.
    fn set_every_window_s_look(&self, look: WindowLook) {
        let mut windows: Vec<String> = self.lock().windows.keys().cloned().collect();
        windows.sort();
        for window in windows {
            // As with a page that can't be told: the window that made the change can't mend
            // another, and the next change of look sets every window's whole look again.
            let _ = self.inner.windows.set_look(&window, &look);
        }
    }

    /// `window`'s page has asked for its document, which it does only once it hears the settings
    /// (`page/src/main.ts`), so it is sent every change from now on, and the settings now if they
    /// changed before (ticket 16's review): a window that opened while they changed has the
    /// settings from before the change in its page's URL, and missed the message, and may have
    /// been still being built when the window look changed, so it is given the look too. Both
    /// while the settings are held, so that a change after it reaches the window after it.
    pub(super) fn page_hears_settings(&self, window: &str) {
        let missed = self.lock().not_hearing.remove(window) == Some(true);
        if missed {
            self.inner.settings.read(|settings| {
                // As in `send_settings_to_every_page`, the next change catches the window up.
                let _ = self.inner.windows.set_look(window, &settings.window_look());
                let _ = self.send(window, Message::Settings(settings.page()));
            });
        }
    }
}
