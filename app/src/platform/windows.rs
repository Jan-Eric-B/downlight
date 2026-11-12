use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, Runtime, WebviewWindow, WebviewWindowBuilder};
use webview2_com::AcceleratorKeyPressedEventHandler;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2AcceleratorKeyPressedEventArgs, ICoreWebView2AcceleratorKeyPressedEventArgs2,
};
use windows_core::Interface as _;
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VK_BROWSER_REFRESH, VK_CONTROL, VK_F5,
};
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

/// Leaves WebView2's reload keys (F5, Ctrl+R, Ctrl+Shift+R, Ctrl+F5, Shift+F5 and a keyboard's
/// Refresh key) to the page, which reloads the document on F5 and Ctrl+R and swallows the rest
/// (ticket 10). Without this, WebView2 loads the page anew on Ctrl+Shift+R even when the page
/// cancels the key, and the window loses its reading position. WebView2's other keys, such as
/// zoom, keep working, which turning off all its browser keys would stop.
///
/// Without the handler only Ctrl+Shift+R still loads the page anew, and the window works
/// otherwise, so a failure to reach the webview or to add the handler is left at that.
pub fn leave_reload_keys_to_the_page(window: &WebviewWindow) {
    let _ = window.with_webview(|webview| {
        let handler = AcceleratorKeyPressedEventHandler::create(Box::new(|_, args| {
            args.map_or(Ok(()), |args| leave_to_the_page_if_reload(&args))
        }));
        let mut token = 0;
        // SAFETY: the handler and the token are valid for the call, and the controller keeps its
        // own reference to the handler. The token isn't needed: the handler lives as long as the
        // webview.
        let _ = unsafe {
            webview
                .controller()
                .add_AcceleratorKeyPressed(&handler, &mut token)
        };
    });
}

fn leave_to_the_page_if_reload(
    args: &ICoreWebView2AcceleratorKeyPressedEventArgs,
) -> windows_core::Result<()> {
    let mut key = 0;
    // SAFETY: `key` is a live u32 that VirtualKey only writes.
    unsafe { args.VirtualKey(&mut key)? };
    // SAFETY: GetKeyState only reads the calling thread's keyboard state. The event comes on the
    // window's thread while it handles the key, so that state is the key's own.
    let ctrl_held = unsafe { GetKeyState(i32::from(VK_CONTROL)) } < 0;
    let reload = key == u32::from(VK_F5)
        || key == u32::from(VK_BROWSER_REFRESH)
        || (key == u32::from(b'R') && ctrl_held);
    if reload {
        // WebView2 still passes the key on to the page.
        let args: ICoreWebView2AcceleratorKeyPressedEventArgs2 = args.cast()?;
        // SAFETY: a plain property setter on a live interface.
        unsafe { args.SetIsBrowserAcceleratorKeyEnabled(false)? };
    }
    Ok(())
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
