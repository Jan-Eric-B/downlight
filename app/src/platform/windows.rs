use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, Runtime, WebviewWindowBuilder};
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW,
};

use crate::APP_NAME;

/// Windows 11's first build. Mica needs it; Windows 10 has no Mica, and Tauri drops the effect
/// there without an error, which would leave a transparent window see-through.
const FIRST_BUILD_WITH_MICA: u32 = 22000;

/// Whether windows get a backdrop, Mica: Windows 11 and later.
pub fn has_backdrop() -> bool {
    windows_version::OsVersion::current().build >= FIRST_BUILD_WITH_MICA
}

/// Puts Mica behind the whole window, for windows where `has_backdrop()` is true. The window and
/// its webview are then transparent, and the page, told so by the app, draws no background of its
/// own, so the document sits on Mica. `Effect::Mica` follows the system's light or dark mode.
pub fn with_backdrop<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    builder
        .transparent(true)
        .effects(EffectsBuilder::new().effect(Effect::Mica).build())
}

/// Where the page loads a custom scheme's URLs from: WebView2 serves custom schemes as
/// `http://<scheme>.localhost`.
pub fn custom_scheme_origin(scheme: &str) -> String {
    format!("http://{scheme}.localhost")
}

/// The path in one case, since Windows file names ignore case. NTFS compares names by
/// upper-casing each character on its own, so a character whose upper case is longer, such as ß,
/// stays as it is.
pub fn fold_case(path: PathBuf) -> PathBuf {
    path.to_string_lossy()
        .chars()
        .map(|c| {
            let mut upper = c.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(single), None) => single,
                _ => c,
            }
        })
        .collect::<String>()
        .into()
}

/// Shows an error in a message box and returns once the reader has dismissed it. It needs no
/// window of Downlight's, so it works when no window could open, and release builds have no
/// console to print to.
pub fn show_error(message: &str) {
    let text = wide(message);
    let caption = wide(APP_NAME);
    // SAFETY: both strings are NUL-terminated UTF-16 buffers that outlive the call, and a null
    // owner window is allowed: the box then belongs to no window.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

/// `s` as the NUL-terminated UTF-16 string Windows' wide APIs take.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// When Windows created this process, so first paint counts the loader's work before `main` too.
pub fn process_start() -> Option<SystemTime> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: GetCurrentProcess returns a pseudo handle that is always valid and needs no
    // closing, and each pointer points to a live FILETIME that GetProcessTimes only writes.
    let succeeded = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if succeeded == 0 {
        return None;
    }
    // A FILETIME counts 100 ns ticks since 1601-01-01, 11,644,473,600 s before the Unix epoch.
    const TICKS_BEFORE_UNIX_EPOCH: u64 = 116_444_736_000_000_000;
    let ticks = (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime);
    let since_epoch = ticks.checked_sub(TICKS_BEFORE_UNIX_EPOCH)?;
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_nanos(since_epoch.checked_mul(100)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_case_upper_cases_every_character() {
        assert_eq!(
            fold_case(PathBuf::from(r"C:\Work\Straße\Ärger.md")),
            PathBuf::from(r"C:\WORK\STRAßE\ÄRGER.MD")
        );
    }

    #[test]
    fn fold_case_keeps_a_character_whose_upper_case_is_longer() {
        assert_eq!(fold_case(PathBuf::from("ß")), PathBuf::from("ß"));
    }
}
