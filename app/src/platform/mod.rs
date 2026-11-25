//! Code that differs per OS, behind one interface, so the macOS and Linux ports touch only this
//! module. v1 implements Windows only.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{
    accent_scale, add_drop_target, add_maximize_button, ask, clipboard_text, custom_scheme_origin,
    draws_own_title_bar, fold_case, follow_accent, has_backdrop, is_missing,
    leave_keys_to_the_page, let_take_foreground, local_time, menu_item_text, notepad,
    open_as_text_file, open_folder, open_url, place_maximize_button, process_start, program_name,
    reveal_file, set_backdrop, show_error, show_window_menu, text_file_program,
    text_file_program_name, when_webview_stops, with_backdrop, with_browser_arguments,
    with_drop_target, write_clipboard_text,
};

/// Other systems name no text file program until their ports find theirs.
#[cfg(not(windows))]
pub fn text_file_program() -> Option<std::path::PathBuf> {
    None
}

/// Other systems name no text file program until their ports; Edit with shows "Default" alone.
#[cfg(not(windows))]
pub fn text_file_program_name() -> Option<String> {
    None
}

/// Other systems name programs by their files until their ports.
#[cfg(not(windows))]
pub fn program_name(_program: &std::path::Path) -> Option<String> {
    None
}

/// Other systems open a document with their text file program with their ports; Edit falls back
/// to the next editor.
#[cfg(not(windows))]
pub fn open_as_text_file(_document: &std::path::Path) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// Other systems name their Notepad with their ports.
#[cfg(not(windows))]
pub fn notepad() -> Option<std::path::PathBuf> {
    None
}

/// Other systems keep their native frame and title bar until their ports decide their own
/// (ADR 0005).
#[cfg(not(windows))]
pub const fn draws_own_title_bar() -> bool {
    false
}

/// Other systems' maximize buttons are their own until their ports; nothing sits behind the
/// page's.
#[cfg(not(windows))]
pub fn add_maximize_button(
    _window: &tauri::WebviewWindow,
    _on_state: impl Fn(crate::title_bar::MaximizeButtonState) + Send + 'static,
) {
}

/// Other systems have no window behind the page's maximize button to place.
#[cfg(not(windows))]
pub fn place_maximize_button(_window: &tauri::WebviewWindow, _rect: crate::title_bar::ButtonRect) {}

/// Other systems keep their native title bar, and with it their window menu, so the page draws no
/// logo to open one from (ADR 0005).
#[cfg(not(windows))]
pub fn show_window_menu(_window: &tauri::WebviewWindow) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

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

/// Other systems hand the shortcut to Tauri's menu until their ports, which shows those it knows.
#[cfg(not(windows))]
pub fn menu_item_text(label: &str, shortcut: Option<&str>) -> (String, Option<String>) {
    (label.to_owned(), shortcut.map(str::to_owned))
}

/// Other systems leave a relaunched Downlight's window where their window managers put it until
/// their ports.
#[cfg(not(windows))]
pub fn let_take_foreground(_process_id: u32) {}

/// Other systems' webviews keep their browser keys until their ports turn them off.
#[cfg(not(windows))]
pub fn leave_keys_to_the_page(_window: &tauri::WebviewWindow) {}

/// Other systems' webviews aren't watched for failing until their ports learn how their webviews
/// tell of it.
#[cfg(not(windows))]
pub fn when_webview_stops(
    _window: &tauri::WebviewWindow,
    _stopped: impl FnOnce(&'static str) + Send + 'static,
) {
}

/// Other systems print the error on stderr until their ports give it a native dialog.
#[cfg(not(windows))]
pub fn show_error(message: &str) {
    eprintln!("{message}");
}

/// Other systems' webviews take no browser arguments (Tauri's `additional_browser_args` is
/// WebView2's); their ports decide how their webviews treat a server's sign-in challenge.
#[cfg(not(windows))]
pub fn with_browser_arguments<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
) -> tauri::WebviewWindowBuilder<'a, R, M> {
    builder
}

/// Other systems ask nothing until their ports, and the answer is no: network shares, which ask,
/// are Windows' paths.
#[cfg(not(windows))]
pub fn ask(_window: &tauri::WebviewWindow, _question: &str) -> bool {
    false
}

/// Other systems get a plain window until their ports.
#[cfg(not(windows))]
pub fn with_backdrop<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
    _backdrop: crate::settings::Backdrop,
) -> tauri::WebviewWindowBuilder<'a, R, M> {
    builder
}

/// Other systems have no backdrop to change until their ports.
#[cfg(not(windows))]
pub fn set_backdrop(
    _window: &tauri::WebviewWindow,
    _backdrop: crate::settings::Backdrop,
) -> tauri::Result<()> {
    Ok(())
}

/// Other systems get a solid background until their ports; macOS vibrancy comes with its port.
#[cfg(not(windows))]
pub fn has_backdrop() -> bool {
    false
}

/// Other systems report no accent color of their own until their ports, so the settings flyout
/// offers Amber alone there.
#[cfg(not(windows))]
pub fn accent_scale() -> Option<crate::settings::AccentScale> {
    None
}

/// Other systems tell of no change of an accent color they don't report.
#[cfg(not(windows))]
pub fn follow_accent(_changed: impl Fn() + Send + 'static) {}

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

/// Other systems keep Tauri's native drop until their ports (ADR 0007): file drops only, which
/// `open::show_when_dropped` takes.
#[cfg(not(windows))]
pub fn with_drop_target<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'a, R, M>,
) -> tauri::WebviewWindowBuilder<'a, R, M> {
    builder
}

/// Other systems' drops come through Tauri's native drop, as `with_drop_target` leaves it: files
/// only, and a text drag gets the no-drop cursor.
#[cfg(not(windows))]
pub fn add_drop_target(
    _window: &tauri::WebviewWindow,
    _dropped: impl Fn(crate::open::Dropped) + Send + 'static,
) {
}

/// Other systems read their clipboards with their ports; Paste says it isn't available yet.
#[cfg(not(windows))]
pub fn clipboard_text() -> crate::paste::Clipboard {
    crate::paste::Clipboard::NotAvailable
}

/// Other systems give a fixed time: Paste isn't available there until their ports, so no file is
/// ever named by it.
#[cfg(not(windows))]
pub const fn local_time() -> crate::paste::LocalTime {
    crate::paste::LocalTime {
        year: 1970,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
    }
}

/// Other systems write their clipboards with their ports; Copy as Markdown says it isn't available
/// yet.
#[cfg(not(windows))]
pub fn write_clipboard_text(
    _window: &tauri::WebviewWindow,
    _text: &str,
) -> Result<(), crate::copy::ClipboardError> {
    Err(crate::copy::ClipboardError::NotAvailable)
}
