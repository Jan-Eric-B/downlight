//! Code that differs per OS, behind one interface, so the macOS and Linux ports touch only this
//! module. v1 implements Windows only.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{
    custom_scheme_origin, fold_case, has_backdrop, is_missing,
    leave_reload_and_find_keys_to_the_page, open_folder, open_url, process_start, reveal_file,
    show_error, with_backdrop,
};

/// Other systems count what Rust counts as not found until their ports.
#[cfg(not(windows))]
pub fn is_missing(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotFound
}

/// Other systems reveal files in their file managers with their ports; the reader is told.
#[cfg(not(windows))]
pub fn reveal_file(_path: &std::path::Path) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// Other systems open folders in their file managers with their ports; the reader is told.
#[cfg(not(windows))]
pub fn open_folder(_path: &std::path::Path) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// Other systems hand URLs to their browsers and mail apps with their ports; the reader is told.
#[cfg(not(windows))]
pub fn open_url(_url: &str) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// Other systems' webviews get their reload and find keys looked at with their ports.
#[cfg(not(windows))]
pub fn leave_reload_and_find_keys_to_the_page(_window: &tauri::WebviewWindow) {}

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
