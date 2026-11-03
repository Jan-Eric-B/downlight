//! Code that differs per OS, behind one interface, so the macOS and Linux ports touch only this
//! module. v1 implements Windows only.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{process_start, with_backdrop};

/// Other systems get a plain window until their ports.
#[cfg(not(windows))]
pub fn with_backdrop<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
) -> tauri::WebviewWindowBuilder<'a, R, M> {
    builder
}

/// Other systems don't report when the process started until their ports; timing then counts
/// from the start of `main`.
#[cfg(not(windows))]
pub fn process_start() -> Option<std::time::SystemTime> {
    None
}
