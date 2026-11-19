//! How Downlight looks (window design ticket 08): the reader's Theme, Backdrop and Accent color.
//! Theme and Backdrop act on the window, not only on the page ([`WindowLook`]), so they reach the
//! windows through the documents module's `Windows` port; every page is sent all three with the
//! settings, for the settings flyout to show them.

use serde::{Deserialize, Serialize};

/// Light or dark (GLOSSARY: Theme). The app sets it on the window, so the backdrop's tint, the
/// title bar and the page's `prefers-color-scheme` follow it together.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    /// Whatever Windows uses, and its changes.
    #[default]
    System,
    Light,
    Dark,
}

/// The material Windows draws behind a window (GLOSSARY: Backdrop), where the OS can show one
/// (`platform::has_backdrop`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Backdrop {
    #[default]
    Mica,
    /// Windows' "tabbed" Mica, a stronger tint of the desktop.
    MicaAlt,
    Acrylic,
    /// No material: the page draws its solid background.
    None,
}

/// Which color scale marks what Downlight adds to a document (GLOSSARY: Accent color). Amber only
/// until window design ticket 09 adds Windows' own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccentColor {
    #[default]
    Amber,
}

/// What of the settings acts on the window itself rather than only on its page: a new window is
/// built with it, and a change of it reaches every window through the `Windows` port.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowLook {
    pub theme: Theme,
    pub backdrop: Backdrop,
}
