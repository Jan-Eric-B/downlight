//! Code that differs per OS, behind one interface, so the macOS and Linux ports touch only this
//! module. v1 implements Windows only.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{document_key, process_start, with_backdrop};

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

/// Other systems resolve the path and compare it as it is until their ports, which know whether
/// their file systems ignore case.
#[cfg(not(windows))]
pub fn document_key(document: &std::path::Path) -> std::path::PathBuf {
    std::fs::canonicalize(document)
        .or_else(|_| std::path::absolute(document))
        .unwrap_or_else(|_| document.to_owned())
}
