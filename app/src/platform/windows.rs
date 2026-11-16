use std::ffi::{OsStr, OsString};
use std::io;
use std::os::windows::ffi::{OsStrExt as _, OsStringExt as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, Runtime, WebviewWindow, WebviewWindowBuilder};
use webview2_com::AcceleratorKeyPressedEventHandler;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2AcceleratorKeyPressedEventArgs, ICoreWebView2AcceleratorKeyPressedEventArgs2,
    ICoreWebView2Controller, ICoreWebView2Settings3,
};
use windows_core::{HRESULT, Interface as _};
use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_INVALID_DRIVE, ERROR_PATH_NOT_FOUND, FILETIME,
};
use windows_sys::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_F12, VK_SHIFT};
use windows_sys::Win32::UI::Shell::{
    ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW, SW_SHOWNORMAL,
};

use crate::APP_NAME;

mod title_bar;

pub use title_bar::{add_maximize_button, place_maximize_button, show_window_menu};

/// Windows 11's first build. Mica needs it; Windows 10 has no Mica, and Tauri drops the effect
/// there without an error, which would leave a transparent window see-through.
const FIRST_BUILD_WITH_MICA: u32 = 22000;

/// Whether the page draws the title bar in place of the native frame (ADR 0005). Windows only, so
/// far; false brings the native title bar back.
pub const fn draws_own_title_bar() -> bool {
    true
}

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
/// stays as it is. The path is folded as UTF-16, which Windows names are, so an unpaired surrogate
/// stays itself and two names that differ only in one don't fold to the same path.
pub fn fold_case(path: PathBuf) -> PathBuf {
    let mut folded = Vec::with_capacity(path.as_os_str().len());
    for unit in char::decode_utf16(path.as_os_str().encode_wide()) {
        match unit {
            Ok(c) => {
                let mut upper = c.to_uppercase();
                let c = match (upper.next(), upper.next()) {
                    (Some(single), None) => single,
                    _ => c,
                };
                folded.extend_from_slice(c.encode_utf16(&mut [0; 2]));
            }
            Err(unpaired) => folded.push(unpaired.unpaired_surrogate()),
        }
    }
    OsString::from_wide(&folded).into()
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
fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

/// Whether `error`, from looking at a path, means nothing is there. Rust counts an offline server
/// (`ERROR_BAD_NETPATH`) and a share that isn't there (`ERROR_BAD_NET_NAME`) as not found too,
/// but a file behind them may exist, so those aren't missing.
pub fn is_missing(error: &io::Error) -> bool {
    error
        .raw_os_error()
        .and_then(|code| u32::try_from(code).ok())
        .is_some_and(|code| {
            [
                ERROR_FILE_NOT_FOUND,
                ERROR_PATH_NOT_FOUND,
                ERROR_INVALID_DRIVE,
            ]
            .contains(&code)
        })
}

/// Shows the file at `path` selected in an Explorer window of its folder (ticket 11). Explorer
/// only selects it, so a link to a program or a script never runs it.
pub fn reveal_file(path: &Path) -> io::Result<()> {
    let path = wide(path);
    with_com(|| {
        // SAFETY: `path` is a NUL-terminated UTF-16 buffer that outlives the call.
        let item = unsafe { ILCreateFromPathW(path.as_ptr()) };
        if item.is_null() {
            // The reader is shown this message, by the title bar's file name and by links.
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Windows has no shell item for it",
            ));
        }
        // SAFETY: `item` is a live absolute item ID list naming the file; with no items to select
        // (0 and null), Explorer opens the file's folder and selects the file itself.
        let opened = unsafe { SHOpenFolderAndSelectItems(item, 0, std::ptr::null(), 0) };
        // SAFETY: `item` came from ILCreateFromPathW and isn't used after this.
        unsafe { ILFree(item) };
        if opened < 0 {
            return Err(hresult_error(opened));
        }
        Ok(())
    })
}

/// The failure `hresult` as an I/O error. One that wraps a Win32 error code gives that code, so
/// the error keeps its kind (a missing file is NotFound); `io::Error` reads a raw OS error as a
/// Win32 code, which an HRESULT isn't, and windows-core's own conversion passes the HRESULT as one.
fn hresult_error(hresult: i32) -> io::Error {
    let win32 = hresult.cast_unsigned() & 0xFFFF;
    if HRESULT::from_win32(win32).0 == hresult {
        io::Error::from_raw_os_error(win32.cast_signed())
    } else {
        io::Error::from_raw_os_error(hresult)
    }
}

/// Opens the folder at `path` in Explorer (ticket 11). With the `explore` verb rather than `open`,
/// a file put where the folder was isn't opened or run: Windows registers `explore` for folders
/// (`HKCR\Folder`) and for no file type, so the shell finds no verb for the file and fails. That
/// rests on the registry, which a program can change for one user without admin rights.
/// `ShellExecuteExW` with the class `Directory` would not help: `explore` isn't one of its verbs,
/// so it fails on a real folder too (ticket 11's review).
pub fn open_folder(path: &Path) -> io::Result<()> {
    shell_execute("explore", path.as_os_str())
}

/// Hands `url`, an `http`, `https` or `mailto` URL that link resolution checked, to the system's
/// browser or mail app (ticket 11).
pub fn open_url(url: &str) -> io::Result<()> {
    shell_execute("open", OsStr::new(url))
}

/// Runs the shell's `verb` on `file`, as Explorer would.
fn shell_execute(verb: &str, file: &OsStr) -> io::Result<()> {
    let (verb, file) = (wide(verb), wide(file));
    with_com(|| {
        // SAFETY: both strings are NUL-terminated UTF-16 buffers that outlive the call; the null
        // window, parameters and folder are allowed (no owner, no arguments, the current folder).
        let instance = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        // ShellExecuteW returns a number, not a real handle: over 32 means it succeeded.
        if instance.addr() > 32 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    })
}

/// Runs `work` with COM started on this thread, as the shell's functions need: some shell
/// extensions they load want a single-threaded apartment. COM is stopped again only if this call
/// started it; a thread already in another apartment keeps it, and the work runs there.
fn with_com<T>(work: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
    // The flags are small positive constants, so the cast keeps their bits.
    let flags = (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).cast_unsigned();
    // SAFETY: the reserved pointer must be null, and each successful call is matched by one
    // CoUninitialize below, on the same thread.
    let started = unsafe { CoInitializeEx(std::ptr::null(), flags) } >= 0;
    let result = work();
    if started {
        // SAFETY: matches the successful CoInitializeEx above, on the same thread.
        unsafe { CoUninitialize() };
    }
    result
}

/// Leaves every key to the page, whose command table says what each one does (ticket 29):
/// WebView2 handles none of its browser keys, such as its reloads, find bar, zoom, print dialog,
/// downloads, caret browsing and Back and Forward, and still passes them on to the page. Text
/// editing and movement keys (Ctrl+C, Ctrl+A, the arrows) aren't browser keys and keep working.
/// Dev builds keep the DevTools keys, as DevTools are on in debug builds only.
///
/// The setting turns the browser keys off as a whole. Tauri reaches the webview only after its page
/// has started loading, though, and Microsoft documents that a setting changed after navigation
/// started applies from the next one, so the handler also turns each browser key off as it is
/// pressed, which holds at once and is where dev builds take the DevTools keys back.
///
/// A window whose browser keys stay on works otherwise, so a failure to reach the webview, to
/// change the setting or to add the handler is left at that.
pub fn leave_keys_to_the_page(window: &WebviewWindow) {
    let _ = window.with_webview(|webview| {
        let controller = webview.controller();
        let _ = turn_off_browser_keys(&controller);
        let handler = AcceleratorKeyPressedEventHandler::create(Box::new(|_, args| {
            args.map_or(Ok(()), |args| turn_off_browser_key(&args))
        }));
        let mut token = 0;
        // SAFETY: the handler and the token are valid for the call, and the controller keeps its
        // own reference to the handler. The token isn't needed: the handler lives as long as the
        // webview.
        let _ = unsafe { controller.add_AcceleratorKeyPressed(&handler, &mut token) };
    });
}

fn turn_off_browser_keys(controller: &ICoreWebView2Controller) -> windows_core::Result<()> {
    // SAFETY: plain getters and a setter on live interfaces, on the window's thread, where Tauri
    // runs `with_webview`'s closure.
    unsafe {
        let settings: ICoreWebView2Settings3 = controller.CoreWebView2()?.Settings()?.cast()?;
        settings.SetAreBrowserAcceleratorKeysEnabled(false)
    }
}

fn turn_off_browser_key(
    args: &ICoreWebView2AcceleratorKeyPressedEventArgs,
) -> windows_core::Result<()> {
    let mut key = 0;
    // SAFETY: `key` is a live u32 that VirtualKey only writes.
    unsafe { args.VirtualKey(&mut key)? };
    // SAFETY: GetKeyState only reads the calling thread's keyboard state. The event comes on the
    // window's thread while it handles the key, so that state is the key's own.
    let held = |modifier: u16| unsafe { GetKeyState(i32::from(modifier)) } < 0;
    let enabled = cfg!(debug_assertions) && is_dev_tools_key(key, held(VK_CONTROL), held(VK_SHIFT));
    let args: ICoreWebView2AcceleratorKeyPressedEventArgs2 = args.cast()?;
    // SAFETY: a plain property setter on a live interface.
    unsafe { args.SetIsBrowserAcceleratorKeyEnabled(enabled) }
}

/// Whether the virtual key `key`, with Ctrl and Shift held or not, opens WebView2's DevTools: F12,
/// or Ctrl+Shift+I.
fn is_dev_tools_key(key: u32, ctrl_held: bool, shift_held: bool) -> bool {
    key == u32::from(VK_F12) || (key == u32::from(b'I') && ctrl_held && shift_held)
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
    use windows_sys::Win32::Foundation::{
        E_FAIL, ERROR_ACCESS_DENIED, ERROR_BAD_NET_NAME, ERROR_BAD_NETPATH,
    };

    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F5;

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

    #[test]
    fn fold_case_keeps_unpaired_surrogates_apart() {
        let name = |units: &[u16]| PathBuf::from(OsString::from_wide(units));
        let (a, upper_a) = (u16::from(b'a'), u16::from(b'A'));
        assert_eq!(fold_case(name(&[0xD800, a])), name(&[0xD800, upper_a]));
        assert_ne!(fold_case(name(&[0xD800, a])), fold_case(name(&[0xD801, a])));
    }

    #[test]
    fn win32_failure_in_an_hresult_keeps_its_kind() {
        let not_found = HRESULT::from_win32(ERROR_FILE_NOT_FOUND).0;
        let error = hresult_error(not_found);
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert_eq!(
            error.raw_os_error(),
            Some(ERROR_FILE_NOT_FOUND.cast_signed())
        );
    }

    #[test]
    fn other_hresult_failure_keeps_its_code() {
        assert_eq!(hresult_error(E_FAIL).raw_os_error(), Some(E_FAIL));
    }

    #[test]
    fn dev_tools_keys_are_f12_and_ctrl_shift_i() {
        let i = u16::from(b'I');
        for (key, ctrl_held, shift_held) in [(VK_F12, false, false), (i, true, true)] {
            assert!(
                is_dev_tools_key(u32::from(key), ctrl_held, shift_held),
                "{key}"
            );
        }
    }

    #[test]
    fn other_keys_are_not_dev_tools_keys() {
        let (i, r) = (u16::from(b'I'), u16::from(b'R'));
        for (key, ctrl_held, shift_held) in [
            (i, false, false),
            (i, true, false),
            (i, false, true),
            (r, true, true),
            (VK_F5, false, false),
        ] {
            assert!(
                !is_dev_tools_key(u32::from(key), ctrl_held, shift_held),
                "{key}"
            );
        }
    }

    #[test]
    fn only_a_missing_file_folder_or_drive_is_missing() {
        let missing = |code: u32| is_missing(&io::Error::from_raw_os_error(code.cast_signed()));
        for code in [
            ERROR_FILE_NOT_FOUND,
            ERROR_PATH_NOT_FOUND,
            ERROR_INVALID_DRIVE,
        ] {
            assert!(missing(code), "{code}");
        }
        // An offline server, a share that isn't there, and a file the reader may not open.
        for code in [ERROR_BAD_NETPATH, ERROR_BAD_NET_NAME, ERROR_ACCESS_DENIED] {
            assert!(!missing(code), "{code}");
        }
    }
}
