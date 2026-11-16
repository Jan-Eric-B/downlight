//! The title bar the page draws in place of the native one (ADR 0005, window design ticket 01):
//! the window's own buttons, and the window behind the page's maximize button that brings back
//! Windows' snap layouts flyout.

use serde::Deserialize;
use tauri::{Manager as _, WebviewWindow, WindowEvent};

use crate::message::{Message, Pages as _};
use crate::platform;

/// Where the page's maximize button is, in physical pixels from the top left of the window's
/// client area, as the page measures it each time it lays the title bar out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct ButtonRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ButtonRect {
    /// Whether `point` is on the button: its right and bottom edges belong to what is beside it.
    pub fn contains(self, (x, y): (i32, i32)) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.width)
            && y < self.y.saturating_add(self.height)
    }
}

/// How the page shows its maximize button while the pointer is on it. On Windows the pointer
/// there reaches the window behind the button, not the page, so the app tells the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaximizeButtonState {
    Normal,
    Hovered,
    Pressed,
}

/// A sample of the argument `place_maximize_button` takes, which the page's tests check the
/// button it measures against (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "maximizeButton": serde_json::to_value(ButtonRect {
            x: 1362,
            y: 0,
            width: 69,
            height: 48,
        })?,
    }))
}

/// Opens the window behind the page's maximize button, which tells the page how to show the
/// button.
pub fn add_maximize_button(window: &WebviewWindow) {
    let app = window.app_handle().clone();
    let label = window.label().to_owned();
    platform::add_maximize_button(window, move |state| {
        // A page that can't be told keeps showing the button as it was, which is all it costs.
        let _ = app.send(&label, Message::MaximizeButton(state));
    });
}

/// The page laid its title bar out, and its maximize button is at `button` now.
#[tauri::command]
pub fn place_maximize_button(window: WebviewWindow, button: ButtonRect) {
    platform::place_maximize_button(&window, button);
}

/// The window's minimize button.
#[tauri::command]
pub fn minimize_window(window: WebviewWindow) -> tauri::Result<()> {
    window.minimize()
}

/// The window's maximize button, for a click that reaches the page's own button. The page draws
/// its title bar only on Windows (`platform::draws_own_title_bar`), and there the window behind
/// the button takes the clicks; this is the fallback while that window isn't there to take them
/// (not created, or not placed yet).
#[tauri::command]
pub fn toggle_maximize_window(window: WebviewWindow) -> tauri::Result<()> {
    if window.is_maximized()? {
        window.unmaximize()
    } else {
        window.maximize()
    }
}

/// Tells the window's page, each time the window's size changes, whether it is maximized now, so
/// the page shows maximize or restore.
pub fn tell_window_state(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::Resized(_) = event
        && let Ok(maximized) = window.is_maximized()
    {
        // Sent on every resize, so a missed one is put right by the next.
        let _ = window
            .app_handle()
            .send(window.label(), Message::WindowState { maximized });
    }
}
