//! What the app tells a window's page, as one typed set of messages (ticket 22). The app sends them
//! through Tauri's events ([`Pages`] on `AppHandle`); the tests record them instead, so what a
//! window's page is told can be checked without a webview.

use tauri::{AppHandle, Emitter as _};

use crate::notice::Notice;
use crate::title_bar::MaximizeButtonState;

/// A message for one window's page. Each goes as the event the page listens for, with a string
/// payload, a notice or none (`Messages` in `page/src/app.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// The window shows another document now, and the page is to ask for it.
    DocumentShown,
    /// The document's file was saved or came back, or the reader asked for a reload, and its new
    /// render has started for the page to ask for. Both this and [`Message::DocumentShown`] only
    /// make the page ask: whether it shows the document or reloads it comes with the answer
    /// (`Documents::handover`), since this can reach the page after the `DocumentShown` of a
    /// document the window was given since (ticket 23).
    Reload,
    /// The document's file was deleted or moved away.
    Gone,
    /// A link in another window named a heading in this window's document, to scroll to now.
    ScrollToHeading(String),
    /// A short message to show the reader for a few seconds, such as why a dropped file didn't
    /// open or why the document's file isn't watched.
    Notice(Notice),
    /// The command, by id, that the reader chose in the window's right-click menu.
    MenuCommand(String),
    /// The window's size changed, and whether it is maximized now, so the page's title bar shows
    /// maximize or restore.
    WindowState { maximized: bool },
    /// How the page's maximize button is to look: the pointer on it reaches the window behind
    /// the button on Windows, not the page (`title_bar.rs`).
    MaximizeButton(MaximizeButtonState),
}

/// What a message carries to the page.
#[derive(Clone, serde::Serialize)]
#[serde(untagged)]
enum Payload<'a> {
    Text(&'a str),
    Notice(&'a Notice),
}

impl Message {
    /// The event the page listens for, and its payload: None goes as `null`.
    fn event(&self) -> (&'static str, Option<Payload<'_>>) {
        let text = |text| Some(Payload::Text(text));
        match self {
            Self::DocumentShown => ("document-shown", None),
            Self::Reload => ("document-changed", text("reload")),
            Self::Gone => ("document-changed", text("gone")),
            Self::ScrollToHeading(heading) => ("scroll-to-heading", text(heading)),
            Self::Notice(notice) => ("notice", Some(Payload::Notice(notice))),
            Self::MenuCommand(id) => ("menu-command", text(id)),
            Self::WindowState { maximized } => (
                "window-state",
                text(if *maximized { "maximized" } else { "restored" }),
            ),
            Self::MaximizeButton(state) => (
                "maximize-button",
                text(match state {
                    MaximizeButtonState::Normal => "normal",
                    MaximizeButtonState::Hovered => "hovered",
                    MaximizeButtonState::Pressed => "pressed",
                }),
            ),
        }
    }
}

/// A sample of each message, by name, as the event and payload the page gets, which the page's
/// tests read (`app/src/contract.rs`, ticket 27).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    use crate::notice::NamedFile;

    // A new message stops the build here: give it a sample below, then name it here.
    let _sampled = |message: &Message| match message {
        Message::DocumentShown
        | Message::Reload
        | Message::Gone
        | Message::ScrollToHeading(_)
        | Message::Notice(_)
        | Message::MenuCommand(_)
        | Message::WindowState { .. }
        | Message::MaximizeButton(_) => {}
    };
    let sample = |message: Message| {
        let (event, payload) = message.event();
        serde_json::json!({ "event": event, "payload": payload })
    };
    let naming_file = Notice::naming(
        "Can't watch ",
        NamedFile::sample(false),
        " for changes, so only F5 reloads it: Access is denied.",
    );
    Ok(serde_json::json!({
        "documentShown": sample(Message::DocumentShown),
        "reload": sample(Message::Reload),
        "gone": sample(Message::Gone),
        "scrollToHeading": sample(Message::ScrollToHeading("install".to_owned())),
        "notice": sample(Message::Notice("Can't watch the document's file".into())),
        "noticeNamingFile": sample(Message::Notice(naming_file)),
        "menuCommand": sample(Message::MenuCommand("open".to_owned())),
        "maximized": sample(Message::WindowState { maximized: true }),
        "restored": sample(Message::WindowState { maximized: false }),
        "maximizeButtonNormal": sample(Message::MaximizeButton(MaximizeButtonState::Normal)),
        "maximizeButtonHovered": sample(Message::MaximizeButton(MaximizeButtonState::Hovered)),
        "maximizeButtonPressed": sample(Message::MaximizeButton(MaximizeButtonState::Pressed)),
    }))
}

/// Where messages for windows' pages go: their webviews in the app, a recorder in the tests.
pub trait Pages: Send + Sync + 'static {
    /// Sends `message` to `window`'s page. A window that has closed gets nothing, and that is no
    /// failure; a failure is a page that is there but can't be given the message.
    fn send(&self, window: &str, message: Message) -> tauri::Result<()>;
}

impl Pages for AppHandle {
    fn send(&self, window: &str, message: Message) -> tauri::Result<()> {
        let (event, payload) = message.event();
        // Tauri 2.12 sends to a closed window's webview nothing and reports Ok, so an error is a
        // live webview that failed to run the event (or a payload that didn't serialize).
        self.emit_to(window, event, payload)
    }
}

/// The tests' [`Pages`].
#[cfg(test)]
pub mod recorder {
    use std::io;
    use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
    use std::time::Duration;

    use super::{Message, Pages};

    /// A message as a [`Recorder`] keeps it: the window it went to, and the message.
    pub type Sent = (String, Message);

    /// Keeps every message sent, in order, for a test to check. Its clones share what they keep.
    #[derive(Clone, Default)]
    pub struct Recorder {
        sent: Arc<(Mutex<Vec<Sent>>, Condvar)>,
        /// Windows whose pages can't be given a message, as a webview that fails to run an event.
        failing: Arc<Mutex<Vec<String>>>,
    }

    impl Recorder {
        /// From now on, every message to `window`'s page fails to send, and isn't kept.
        pub fn fail_sends_to(&self, window: &str) {
            self.failing
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(window.to_owned());
        }

        /// Every message sent so far.
        pub fn sent(&self) -> Vec<Sent> {
            self.lock().clone()
        }

        /// Waits until the messages sent so far are `done`, or `timeout` has passed, and returns
        /// every message sent by then. Messages sent from a watch's thread arrive in their own
        /// time.
        pub fn wait_until(&self, timeout: Duration, done: impl Fn(&[Sent]) -> bool) -> Vec<Sent> {
            let (sent, arrived) = &*self.sent;
            let sent = sent.lock().unwrap_or_else(PoisonError::into_inner);
            let (sent, _) = arrived
                .wait_timeout_while(sent, timeout, |sent| !done(sent))
                .unwrap_or_else(PoisonError::into_inner);
            sent.clone()
        }

        fn lock(&self) -> MutexGuard<'_, Vec<Sent>> {
            self.sent.0.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    impl Pages for Recorder {
        fn send(&self, window: &str, message: Message) -> tauri::Result<()> {
            let failing = self
                .failing
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .iter()
                .any(|failing| failing == window);
            if failing {
                return Err(io::Error::other("the page can't be given the message").into());
            }
            self.lock().push((window.to_owned(), message));
            self.sent.1.notify_all();
            Ok(())
        }
    }
}
