//! What the app tells a window's page, as one typed set of messages (ticket 22). The app sends them
//! through Tauri's events ([`Pages`] on `AppHandle`); the tests record them instead, so what a
//! window's page is told can be checked without a webview.

use tauri::{AppHandle, Emitter as _};

use crate::documents::{Showing, WindowError};
use crate::notice::Notice;
use crate::title_bar::MaximizeButtonState;

/// A message for one window's page. Each goes as the event the page listens for, with a string
/// payload, a showing, a notice or none (`Messages` in `page/src/app.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// The page is to ask for its document: the window shows another document now, or the
    /// document's file was saved or came back and its new render has started. One message for
    /// both (ticket 26), since whether the page shows the document or reloads it comes with the
    /// answer (`Documents::handover`): a save to the document the window has just left can reach
    /// the page after the message for the document it was given since (ticket 23).
    AskForDocument,
    /// The file of the document the window shows as this showing was deleted or moved away. It
    /// names the showing, so a page whose view shows another one tells it apart (ticket 26).
    Gone(Showing),
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
    Showing(Showing),
    Notice(&'a Notice),
}

impl Message {
    /// The event the page listens for, and its payload: None goes as `null`.
    fn event(&self) -> (&'static str, Option<Payload<'_>>) {
        let text = |text| Some(Payload::Text(text));
        match self {
            Self::AskForDocument => ("ask-for-document", None),
            Self::Gone(showing) => ("document-gone", Some(Payload::Showing(*showing))),
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
        Message::AskForDocument
        | Message::Gone(_)
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
        "askForDocument": sample(Message::AskForDocument),
        // The showing of the `render_document` samples, so the page's tests can tell a `gone`
        // for the showing its view shows.
        "gone": sample(Message::Gone(Showing::sample())),
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

/// Where messages for windows' pages go: their webviews in the app, a recorder in the tests
/// (`documents::recorder`). What is done to the windows themselves goes through the documents
/// module's `Windows`.
pub trait Pages: Send + Sync + 'static {
    /// Sends `message` to `window`'s page. A window that has closed gets nothing, and that is no
    /// failure; a failure is a page that is there but can't be given the message.
    fn send(&self, window: &str, message: Message) -> Result<(), WindowError>;
}

impl Pages for AppHandle {
    fn send(&self, window: &str, message: Message) -> Result<(), WindowError> {
        let (event, payload) = message.event();
        // Tauri 2.12 sends to a closed window's webview nothing and reports Ok, so an error is a
        // live webview that failed to run the event (or a payload that didn't serialize).
        Ok(self.emit_to(window, event, payload)?)
    }
}
