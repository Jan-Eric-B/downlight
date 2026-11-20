//! Changing the reader's settings and sending them to every page (window design ticket 12): a
//! change reaches every page that hears it, and is held for a page that doesn't yet, until it
//! first asks for its document (ticket 16's review), and so does a change of Windows' accent
//! color, which the pages get with the settings (window design ticket 09). A change of the window
//! look (Theme and Backdrop, window design ticket 08) reaches every window through the
//! [`Windows`] port too. Kept apart from the table, which only records which pages hear the
//! settings yet (its `not_hearing`).

use std::sync::{MutexGuard, PoisonError};

use super::Documents;
#[cfg(doc)]
use super::Windows;
use crate::message::Message;
use crate::settings::{AccentScale, PageSettings, Settings, SettingsError, WindowLook};

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
                let scale = *self.windows_accent();
                let shown = updated.settings.page(scale);
                if updated.before.page(scale) != shown {
                    self.send_settings_to_every_page(shown);
                }
            })
            .saved
    }

    /// Windows' accent colors may have changed (window design ticket 09): the scale is read
    /// again, and every page is sent the settings with it if it changed, as for a change of the
    /// settings, so that a page whose reader chose Windows' recolors at once and one that chose
    /// Amber has it ready. Windows tells of a change more than once, and of changes to other
    /// colors, so an unchanged scale sends nothing.
    ///
    /// Read while the settings are held, so that of two calls at once, on Windows' threads or at
    /// startup, the scale stored last is the one read last. A read that fails keeps the scale the
    /// pages have: None reaches them only from an OS that never tells one, and a page's settings
    /// flyout decides once from it whether to offer Windows' accent color.
    pub fn change_windows_accent(&self) {
        self.inner.settings.read(|settings| {
            let Some(scale) = self.inner.windows.accent_scale() else {
                return;
            };
            let before = self.windows_accent().replace(scale);
            if before != Some(scale) {
                self.send_settings_to_every_page(settings.page(Some(scale)));
            }
        });
    }

    /// What the pages show of `settings`, with Windows' accent scale now. Under the settings'
    /// lock it is what they are sent next; outside it, as for a new window's URL (`open`), a
    /// change of the scale meanwhile is sent to the window's page when it first asks
    /// ([`Documents::page_hears_settings`]).
    pub(super) fn page_settings(&self, settings: &Settings) -> PageSettings {
        settings.page(*self.windows_accent())
    }

    fn windows_accent(&self) -> MutexGuard<'_, Option<AccentScale>> {
        // The scale is replaced whole, so a panic elsewhere never leaves it half-changed.
        self.inner
            .windows_accent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
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
                let _ = self.send(window, Message::Settings(self.page_settings(settings)));
            });
        }
    }
}
