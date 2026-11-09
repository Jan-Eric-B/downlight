//! The right-click menu on the document. The page says what is in it (`page/src/commands.ts`),
//! so a later command is added to the menu in that one place; the app shows the entries as a
//! native menu and tells the page which command the reader chose.

use serde::Deserialize;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter as _, WebviewWindow};

/// The event that tells a window's page which command the reader chose in its menu.
const MENU_COMMAND: &str = "menu-command";

/// One line of the menu, as the page describes it.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MenuEntry {
    /// A command the page carries out, with the shortcut shown beside it, such as `Ctrl+O`.
    Command {
        id: String,
        label: String,
        shortcut: Option<String>,
    },
    Separator,
    /// The system's own Copy, which copies the selection as Ctrl+C does.
    Copy,
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
    if let Some((window, command)) = chosen(event.id().as_ref()) {
        // The only way to fail is that the window has closed, and then nothing is left to do.
        let _ = app.emit_to(window, MENU_COMMAND, command);
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
