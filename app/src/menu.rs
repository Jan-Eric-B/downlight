//! The right-click menu on the document. The page says what is in it (`page/src/commands.ts`),
//! so a later command is added to the menu in that one place; the app shows the entries as a
//! native menu and tells the page which command the reader chose.

use serde::Deserialize;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Manager as _, WebviewWindow};

use crate::documents::Documents;
use crate::message::Message;

/// One line of the menu, as the page describes it. Serialized only for its contract samples.
#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MenuEntry {
    /// A command the page carries out, with the shortcut shown beside it, such as `Ctrl+O`.
    Command {
        id: String,
        label: String,
        // The page leaves out the shortcut of a command that has none, so its sample does too.
        #[cfg_attr(test, serde(skip_serializing_if = "Option::is_none"))]
        shortcut: Option<String>,
    },
    Separator,
    /// The system's own Copy, which copies the selection as Ctrl+C does.
    Copy,
}

/// A sample of each kind of menu entry the app takes, by name, which the page's tests check the
/// entries it sends `show_document_menu` against (`app/src/contract.rs`, ticket 27).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A new kind of entry stops the build here: give it a sample below, then name it here.
    let _sampled = |entry: &MenuEntry| match entry {
        MenuEntry::Command { .. } | MenuEntry::Separator | MenuEntry::Copy => {}
    };
    let open = |shortcut: Option<&str>| MenuEntry::Command {
        id: "open".to_owned(),
        label: "Open...".to_owned(),
        shortcut: shortcut.map(str::to_owned),
    };
    Ok(serde_json::json!({
        "command": serde_json::to_value(open(Some("Ctrl+O")))?,
        "commandWithoutShortcut": serde_json::to_value(open(None))?,
        "separator": serde_json::to_value(MenuEntry::Separator)?,
        "copy": serde_json::to_value(MenuEntry::Copy)?,
    }))
}

/// Shows the page's menu at the pointer. It runs on the main thread, as a native menu must, and
/// returns once the menu has closed.
#[tauri::command]
pub fn show_document_menu(window: WebviewWindow, entries: Vec<MenuEntry>) -> tauri::Result<()> {
    let menu = Menu::new(&window)?;
    for entry in entries {
        match entry {
            MenuEntry::Command {
                id,
                label,
                shortcut,
            } => menu.append(&MenuItem::with_id(
                &window,
                item_id(window.label(), &id),
                label,
                true,
                shortcut,
            )?)?,
            MenuEntry::Separator => menu.append(&PredefinedMenuItem::separator(&window)?)?,
            MenuEntry::Copy => menu.append(&PredefinedMenuItem::copy(&window, Some("Copy"))?)?,
        }
    }
    window.popup_menu(&menu)
}

/// Passes the reader's choice to the page of the window whose menu it was.
pub fn forward_choice(app: &AppHandle, event: MenuEvent) {
    forward(&app.state::<Documents>(), event.id().as_ref());
}

/// Passes the choice of the menu item `item_id` to the page of the window it names.
fn forward(documents: &Documents, item_id: &str) {
    if let Some((window, command)) = chosen(item_id) {
        // A menu choice arrives as an app event that nobody waits on, so nobody is left to tell.
        let _ = documents.send(window, Message::MenuCommand(command.to_owned()));
    }
}

/// Tauri reports a menu choice to the whole app with only the item's id, so the id names the
/// window as well as the command. Window labels never contain the separator.
fn item_id(window: &str, command: &str) -> String {
    format!("{window}/{command}")
}

/// The window and the command an item id names.
fn chosen(item_id: &str) -> Option<(&str, &str)> {
    item_id.split_once('/')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::recorder::recorded;

    #[test]
    fn choice_goes_to_the_page_of_the_window_whose_menu_it_was() {
        let (documents, recorder) = recorded();
        forward(&documents, &item_id("document-3", "open"));
        forward(&documents, "quit");
        assert_eq!(
            recorder.sent(),
            [(
                "document-3".to_owned(),
                Message::MenuCommand("open".to_owned())
            )]
        );
    }

    #[test]
    fn chosen_item_names_its_window_and_command() {
        assert_eq!(
            chosen(&item_id("document-3", "open")),
            Some(("document-3", "open"))
        );
    }

    #[test]
    fn command_with_a_slash_keeps_its_whole_name() {
        assert_eq!(
            chosen(&item_id("document-3", "zoom/in")),
            Some(("document-3", "zoom/in"))
        );
    }

    #[test]
    fn id_of_another_menu_names_no_window() {
        assert_eq!(chosen("quit"), None);
    }
}
