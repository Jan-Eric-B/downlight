//! How Downlight looks (window design ticket 08): the reader's Theme, Backdrop and Accent color,
//! and the scale of Windows' accent color (ticket 09). Theme and Backdrop act on the window, not
//! only on the page ([`WindowLook`]), so they reach the windows through the documents module's
//! `Windows` port; every page is sent all three with the settings, for the settings flyout to show
//! them, and Windows' scale with them, for the page to color with when the reader chose it.

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

/// Which color scale marks what Downlight adds to a document (GLOSSARY: Accent color).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccentColor {
    /// Downlight's own scale, which the page's stylesheet holds (`page/src/accent.css`).
    #[default]
    Amber,
    /// Windows' accent color, its [`AccentScale`], following Windows when it changes (window
    /// design ticket 09). Offered only where the OS has one (`platform::accent_scale`).
    Windows,
}

/// An accent color's seven shades, Light 3 to Dark 3 (spec: Accent color), keyed as the page's
/// stylesheet names its `--accent-<shade>` variables, so the page sets each by its key
/// (`ACCENT_SHADES` in `page/src/app.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AccentScale {
    #[serde(rename = "light-3")]
    pub light_3: Rgb,
    #[serde(rename = "light-2")]
    pub light_2: Rgb,
    #[serde(rename = "light-1")]
    pub light_1: Rgb,
    pub base: Rgb,
    #[serde(rename = "dark-1")]
    pub dark_1: Rgb,
    #[serde(rename = "dark-2")]
    pub dark_2: Rgb,
    #[serde(rename = "dark-3")]
    pub dark_3: Rgb,
}

impl AccentScale {
    /// A scale for the contract samples and the tests: Windows' default blue, as Windows 11 gives
    /// its shades, off amber's so that a page showing amber instead fails its test.
    #[cfg(test)]
    pub const fn sample() -> Self {
        Self {
            light_3: Rgb([0x99, 0xeb, 0xff]),
            light_2: Rgb([0x4c, 0xc2, 0xff]),
            light_1: Rgb([0x00, 0x91, 0xf8]),
            base: Rgb([0x00, 0x78, 0xd4]),
            dark_1: Rgb([0x00, 0x67, 0xc0]),
            dark_2: Rgb([0x00, 0x3e, 0x92]),
            dark_3: Rgb([0x00, 0x1a, 0x68]),
        }
    }
}

/// An opaque color, red, green and blue, which the page takes as CSS's `#rrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub [u8; 3]);

impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let [red, green, blue] = self.0;
        serializer.collect_str(&format_args!("#{red:02x}{green:02x}{blue:02x}"))
    }
}

/// What of the settings acts on the window itself rather than only on its page: a new window is
/// built with it, and a change of it reaches every window through the `Windows` port.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowLook {
    pub theme: Theme,
    pub backdrop: Backdrop,
}
