//! Code that differs per OS, behind one interface, so the macOS and Linux ports touch only this
//! module. v1 implements Windows only.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{
    custom_scheme_origin, fold_case, has_backdrop, process_start, show_error, with_backdrop,
};

/// Other systems print the error on stderr until their ports give it a native dialog.
#[cfg(not(windows))]
pub fn show_error(message: &str) {
    eprintln!("{message}");
}

/// Other systems get a plain window until their ports.
#[cfg(not(windows))]
pub fn with_backdrop<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
) -> tauri::WebviewWindowBuilder<'a, R, M> {
    builder
}

/// Other systems get a solid background until their ports; macOS vibrancy comes with its port.
#[cfg(not(windows))]
pub fn has_backdrop() -> bool {
    false
}

/// Other systems don't report when the process started until their ports; timing then counts
/// from the start of `main`.
#[cfg(not(windows))]
pub fn process_start() -> Option<std::time::SystemTime> {
    None
}

/// Other systems compare paths as they are until their ports, which know whether their file
/// systems ignore case.
#[cfg(not(windows))]
pub fn fold_case(path: std::path::PathBuf) -> std::path::PathBuf {
    path
}

/// The webviews of macOS and Linux serve custom schemes as `<scheme>://localhost`.
#[cfg(not(windows))]
pub fn custom_scheme_origin(scheme: &str) -> String {
    format!("{scheme}://localhost")
}
