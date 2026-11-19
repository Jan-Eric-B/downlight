use std::ffi::{OsStr, OsString};
use std::io;
use std::os::windows::ffi::{OsStrExt as _, OsStringExt as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tauri::utils::config::WindowEffectsConfig;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, Runtime, WebviewWindow, WebviewWindowBuilder};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
    ICoreWebView2AcceleratorKeyPressedEventArgs, ICoreWebView2AcceleratorKeyPressedEventArgs2,
    ICoreWebView2Controller, ICoreWebView2Settings3,
};
use webview2_com::{AcceleratorKeyPressedEventHandler, ProcessFailedEventHandler};
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
    ASSOCF, ASSOCF_INIT_BYEXENAME, ASSOCF_INIT_IGNOREUNKNOWN, ASSOCF_NOTRUNCATE, ASSOCSTR,
    ASSOCSTR_EXECUTABLE, ASSOCSTR_FRIENDLYAPPNAME, AssocQueryStringW, ILCreateFromPathW, ILFree,
    SEE_MASK_CLASSNAME, SEE_MASK_FLAG_LOG_USAGE, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC,
    SHELLEXECUTEINFOW, SHOpenFolderAndSelectItems, ShellExecuteExW, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, IDYES, MB_DEFBUTTON2, MB_ICONERROR, MB_ICONWARNING, MB_OK,
    MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO, MessageBoxW, SW_SHOWNORMAL,
};

use crate::APP_NAME;
use crate::settings::Backdrop;

mod title_bar;

pub use title_bar::{add_maximize_button, place_maximize_button, show_window_menu};

/// Windows 11's first build. Mica needs it; Windows 10 has no Mica, and Tauri drops the effect
/// there without an error, which would leave a transparent window see-through.
const FIRST_BUILD_WITH_MICA: u32 = 22000;

/// A menu item's text with its `shortcut` shown beside it, as Windows writes it after a tab, and
/// the shortcut for Tauri's menu, which is None. Written into the text rather than handed to muda,
/// which takes only shortcuts it has a key code for and drops the rest without a word, such as
/// zoom's `Ctrl+Plus` (ticket 16); in a popup menu the shortcut is only shown, never pressed.
pub fn menu_item_text(label: &str, shortcut: Option<&str>) -> (String, Option<String>) {
    match shortcut {
        Some(shortcut) => (format!("{label}\t{shortcut}"), None),
        None => (label.to_owned(), None),
    }
}

/// Whether the page draws the title bar in place of the native frame (ADR 0005). Windows only, so
/// far; false brings the native title bar back.
pub const fn draws_own_title_bar() -> bool {
    true
}

/// The first build that sets a backdrop through `DWMWA_SYSTEMBACKDROP_TYPE`, which Mica Alt needs
/// (window-vibrancy's `apply_tabbed`). Windows 11 builds before it show Mica through an older,
/// undocumented attribute and have no Mica Alt.
const FIRST_BUILD_WITH_BACKDROP_TYPE: u32 = 22523;

/// Whether windows get a backdrop, Mica: Windows 11 and later.
pub fn has_backdrop() -> bool {
    windows_version::OsVersion::current().build >= FIRST_BUILD_WITH_MICA
}

/// The effect that draws `backdrop`, or None for no backdrop. Mica Alt falls back to Mica where
/// the system has no Mica Alt, rather than leave the page, which draws no background on a
/// backdrop, on nothing.
fn effect_of(backdrop: Backdrop) -> Option<Effect> {
    match backdrop {
        Backdrop::Mica => Some(Effect::Mica),
        Backdrop::MicaAlt
            if windows_version::OsVersion::current().build >= FIRST_BUILD_WITH_BACKDROP_TYPE =>
        {
            Some(Effect::Tabbed)
        }
        Backdrop::MicaAlt => Some(Effect::Mica),
        Backdrop::Acrylic => Some(Effect::Acrylic),
        Backdrop::None => None,
    }
}

/// Builds the window for a backdrop where the OS can show one (`has_backdrop()`), whatever the
/// reader's Backdrop, and starts it with `backdrop`'s effect; elsewhere, as on Windows 10, it
/// leaves the builder as it is, for a solid, opaque window. The window and its webview are
/// transparent, and the page, told so by the app, draws no background of its own unless the
/// reader's Backdrop is None, so the document sits on the backdrop. The effects follow the
/// window's theme, which tao sets from the reader's Theme or from Windows, and also gives the
/// webview as its `prefers-color-scheme`, so the backdrop and the page agree.
///
/// No redirection surface (`WS_EX_NOREDIRECTIONBITMAP`), so no white surface shows behind the page
/// before the window first paints it clear, a paint tao holds back while the event loop is busy,
/// as during `setup` (window design ticket 13).
///
/// This and `transparent` can only be set when the window is built, which is why they don't
/// depend on the reader's choice: a live change of Backdrop changes only the effect
/// ([`set_backdrop`], window design ticket 08).
pub fn with_backdrop<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
    backdrop: Backdrop,
) -> WebviewWindowBuilder<'a, R, M> {
    if !has_backdrop() {
        return builder;
    }
    let builder = builder.transparent(true).no_redirection_bitmap(true);
    match effect_of(backdrop) {
        Some(effect) => builder.effects(EffectsBuilder::new().effect(effect).build()),
        None => builder,
    }
}

/// Changes `window`'s backdrop to `backdrop` while it is open, where the OS can show one. Tauri
/// applies effects on the main thread and only posts them from here, so this never waits for it.
pub fn set_backdrop(window: &WebviewWindow, backdrop: Backdrop) -> tauri::Result<()> {
    if !has_backdrop() {
        return Ok(());
    }
    // Windows 11 builds before 22523 draw Acrylic and Mica through different attributes, so
    // switching from one to the other there would leave the first on: the effect is cleared
    // first. Later builds set one attribute, which the next effect replaces.
    let replaces = windows_version::OsVersion::current().build >= FIRST_BUILD_WITH_BACKDROP_TYPE;
    let effect = effect_of(backdrop);
    if !replaces || effect.is_none() {
        window.set_effects(None::<WindowEffectsConfig>)?;
    }
    match effect {
        Some(effect) => window.set_effects(EffectsBuilder::new().effect(effect).build()),
        None => Ok(()),
    }
}

/// The browser arguments of every webview, the same for each window: windows in one process share
/// one WebView2 browser, which refuses a webview whose arguments differ from those it runs with.
/// A second launch hands its documents to the running process (ADR 0001) and builds no webview,
/// so its own `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` changes nothing (ticket 19).
///
/// - wry's own defaults, which setting arguments replaces: no mini menu on selected text, no PDF
///   menu, and no SmartScreen check of each URL.
/// - No server gets the reader's Windows sign-in without asking (ticket 20). Chromium answers an
///   NTLM or Negotiate challenge with it on its own for any host Windows counts as intranet
///   (`URLACTION_CREDENTIALS_USE`, a dotless name such as `http://fileserver/` by default) unless
///   an allowlist is set; an empty one counts as unset, so the list names a host that can never
///   resolve (RFC 6761). A proxy's challenge is still answered.
const BROWSER_ARGUMENTS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
     --auth-server-allowlist=downlight.invalid";

/// Starts the window's webview with [`BROWSER_ARGUMENTS`].
pub fn with_browser_arguments<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    builder.additional_browser_args(BROWSER_ARGUMENTS)
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
/// console to print to. Topmost, since belonging to no window it would otherwise open behind a
/// window kept on top (window design ticket 07), as a second launch's failures do.
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
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST,
        );
    }
}

/// Asks the reader a yes or no `question` in a message box that belongs to `window`, which waits
/// for the answer, and says whether the answer was yes. No is the default button, so a stray
/// Enter answers no. Meant for a blocking thread: the box runs its own message loop there, and
/// the main thread and the other windows keep answering.
pub fn ask(window: &WebviewWindow, question: &str) -> bool {
    // Without its window the box would belong to no window, and the reader could miss it.
    let Ok(owner) = window.hwnd() else {
        return false;
    };
    let text = wide(question);
    let caption = wide(APP_NAME);
    // SAFETY: both strings are NUL-terminated UTF-16 buffers that outlive the call, and a window
    // that has closed meanwhile only makes the call fail, which counts as no.
    let answer = unsafe {
        MessageBoxW(
            owner.0,
            text.as_ptr(),
            caption.as_ptr(),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
        )
    };
    answer == IDYES
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

/// The extension whose program is the text file program.
const TEXT_FILE: &str = ".txt";

/// The text file program's file, or None when the system names none: no program opens `.txt`
/// files, or the one that does names no file, as some packaged programs don't. Only asks the
/// shell; nothing starts.
pub fn text_file_program() -> Option<PathBuf> {
    // Fails rather than naming the "Unknown" program that asks the reader to pick one.
    let program = PathBuf::from(assoc_string(
        ASSOCF_INIT_IGNOREUNKNOWN,
        ASSOCSTR_EXECUTABLE,
        OsStr::new(TEXT_FILE),
        Some("open"),
    )?);
    program.is_file().then_some(program)
}

/// The text file program's name as Windows shows it, such as "Notepad", for Edit with's Default
/// (window design ticket 08). Also for a packaged program, whose file the system doesn't name.
pub fn text_file_program_name() -> Option<String> {
    let name = assoc_string(
        ASSOCF_INIT_IGNOREUNKNOWN,
        ASSOCSTR_FRIENDLYAPPNAME,
        OsStr::new(TEXT_FILE),
        Some("open"),
    )?;
    Some(name.to_string_lossy().into_owned())
}

/// The name of the program at `program`, an `.exe`, as Windows shows it: its description, such
/// as "Notepad++ : a free (GNU) source code editor", or None when it has none.
pub fn program_name(program: &Path) -> Option<String> {
    let name = assoc_string(
        ASSOCF_INIT_BYEXENAME,
        ASSOCSTR_FRIENDLYAPPNAME,
        program.as_os_str(),
        None,
    )?;
    Some(name.to_string_lossy().into_owned())
}

/// The shell's string `what` for the association `assoc` (an extension, or a program's file with
/// `ASSOCF_INIT_BYEXENAME`), with its `verb`, or None when the shell has none. Only asks the
/// shell; nothing starts.
fn assoc_string(
    init: ASSOCF,
    what: ASSOCSTR,
    assoc: &OsStr,
    verb: Option<&str>,
) -> Option<OsString> {
    let assoc = wide(assoc);
    let verb = verb.map(wide);
    let verb = verb.as_ref().map_or(std::ptr::null(), |verb| verb.as_ptr());
    let flags = init | ASSOCF_NOTRUNCATE;
    let query = |buffer: *mut u16, length: &mut u32| {
        // SAFETY: `assoc` and `verb`, when not null, are NUL-terminated UTF-16 buffers that
        // outlive the call; a null verb is allowed. `buffer` is null, which asks only for the
        // length, or holds `length` characters.
        unsafe { AssocQueryStringW(flags, what, assoc.as_ptr(), verb, buffer, length) }
    };
    with_com(|| {
        let mut length = 0;
        // S_FALSE (1): no buffer was given, and `length` is the one needed.
        if query(std::ptr::null_mut(), &mut length) != 1 || length == 0 {
            return Ok(None);
        }
        let mut buffer = vec![0; usize::try_from(length).unwrap_or(0)];
        if query(buffer.as_mut_ptr(), &mut length) < 0 {
            return Ok(None);
        }
        let end = buffer
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(buffer.len());
        Ok((end > 0).then(|| OsString::from_wide(&buffer[..end])))
    })
    .ok()
    .flatten()
}

/// Windows' own Notepad, by its full path, so no `notepad.exe` beside Downlight or on the path
/// starts in its place.
pub fn notepad() -> Option<PathBuf> {
    let windows = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    Some(Path::new(&windows).join("System32").join("notepad.exe"))
}

/// Opens `document` with the program for `.txt` files, whatever its own extension. It starts
/// through the shell, which follows the reader's choice of program for `.txt` files and starts a
/// packaged one, such as Windows 11's Notepad, as no command line can.
pub fn open_as_text_file(document: &Path) -> io::Result<()> {
    let (class, file) = (wide(TEXT_FILE), wide(document));
    with_com(|| {
        let mut info = SHELLEXECUTEINFOW {
            // The struct's size is a small constant, so the cast keeps it.
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            // NOASYNC: the calling thread has no message loop to finish the launch with. NO_UI: a
            // missing program fails without a dialog, so Edit falls back to Notepad.
            fMask: SEE_MASK_CLASSNAME
                | SEE_MASK_NOASYNC
                | SEE_MASK_FLAG_NO_UI
                | SEE_MASK_FLAG_LOG_USAGE,
            lpFile: file.as_ptr(),
            lpClass: class.as_ptr(),
            nShow: SW_SHOWNORMAL,
            ..Default::default()
        };
        // SAFETY: `info` is a live SHELLEXECUTEINFOW with its size set, whose strings are
        // NUL-terminated UTF-16 buffers that outlive the call; the null verb is the default one,
        // and the other null members are allowed (no owner, parameters or folder).
        let launched = unsafe { ShellExecuteExW(&mut info) };
        if launched == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
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

/// Calls `stopped` with why, once, when `window`'s webview can no longer show its page: WebView2's
/// browser process ended, which closes the webview, or the render process that drew the page did,
/// which leaves WebView2's error page in its place, with no title bar to close the window by
/// (ticket 37). Called on the window's thread, as WebView2 tells of the failure there.
///
/// Reaching the webview or adding the handler fails only once the webview has closed or the app is
/// ending, with no page left to watch, so a failure leaves the window unwatched, as it was before
/// ticket 37: the reader closes it.
pub fn when_webview_stops(
    window: &WebviewWindow,
    stopped: impl FnOnce(&'static str) + Send + 'static,
) {
    let _ = window.with_webview(move |webview| {
        let mut stopped = Some(stopped);
        let handler = ProcessFailedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else {
                return Ok(());
            };
            let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
            // SAFETY: `kind` is a live value that ProcessFailedKind only writes.
            unsafe { args.ProcessFailedKind(&mut kind)? };
            // Both processes ending raise the event for one webview, in either order, and the
            // window is closed only once.
            if let Some(why) = why_the_window_stops(kind)
                && let Some(stopped) = stopped.take()
            {
                stopped(why);
            }
            Ok(())
        }));
        let mut token = 0;
        // SAFETY: the handler and the token are valid for the call, and the webview keeps its own
        // reference to the handler. The token isn't needed: the handler lives as long as the
        // webview.
        let _ = unsafe {
            webview
                .controller()
                .CoreWebView2()
                .and_then(|webview| webview.add_ProcessFailed(&handler, &mut token))
        };
    });
}

/// Why a WebView2 process failure of `kind` leaves the window unable to show its page, or None
/// when WebView2 recovers from it on its own, as from a GPU process ending, or the failure passes,
/// as an unresponsive render process does once a long task ends.
fn why_the_window_stops(kind: COREWEBVIEW2_PROCESS_FAILED_KIND) -> Option<&'static str> {
    match kind {
        COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED => {
            Some("WebView2's browser process ended")
        }
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED => {
            Some("WebView2's render process ended")
        }
        _ => None,
    }
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

/// Lets the process `process_id`, which this one started, bring its window to the front (ticket
/// 34's relaunch). Windows lets a process take the foreground when the foreground process started
/// it, which holds for a launch from the shell but not for one this windowless process started,
/// whose window would come up behind the shell's; a process that may take the foreground can
/// hand that right on (`AllowSetForegroundWindow`). Windows refuses when this process may not
/// either, and the window then comes up as it would have anyway, so the refusal is left at that.
pub fn let_take_foreground(process_id: u32) {
    // SAFETY: AllowSetForegroundWindow takes a plain process ID and touches no memory of ours.
    unsafe { AllowSetForegroundWindow(process_id) };
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
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE,
        COREWEBVIEW2_PROCESS_FAILED_KIND_SANDBOX_HELPER_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_UNKNOWN_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED,
    };
    use windows_sys::Win32::Foundation::{
        E_FAIL, ERROR_ACCESS_DENIED, ERROR_BAD_NET_NAME, ERROR_BAD_NETPATH,
    };

    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F5;

    use super::*;

    /// The value of the browser argument `--name=value`, if it is given.
    fn browser_argument(name: &str) -> Option<&'static str> {
        BROWSER_ARGUMENTS
            .split_ascii_whitespace()
            .find_map(|argument| argument.strip_prefix(&format!("--{name}=")))
    }

    #[test]
    fn webview_answers_no_servers_sign_in_challenge_on_its_own() {
        // An empty allowlist would fall back to Windows' zones, which let intranet hosts in.
        let allowlist = browser_argument("auth-server-allowlist").unwrap_or_default();
        assert!(
            !allowlist.is_empty()
                && allowlist
                    .split(',')
                    .all(|host| host.ends_with(".invalid") && !host.contains('*')),
            "{allowlist:?}"
        );
    }

    #[test]
    fn webview_keeps_wry_s_default_browser_arguments() {
        assert_eq!(
            browser_argument("disable-features"),
            Some("msWebOOUI,msPdfOOUI,msSmartScreenProtection")
        );
    }

    #[test]
    fn menu_item_shows_any_shortcut_after_a_tab_as_windows_writes_it() {
        assert_eq!(
            menu_item_text("Zoom in", Some("Ctrl+Plus")),
            ("Zoom in\tCtrl+Plus".to_owned(), None)
        );
        assert_eq!(
            menu_item_text("Choose editor...", None),
            ("Choose editor...".to_owned(), None)
        );
    }

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
    fn window_stops_when_webview2_s_browser_or_render_process_ends() {
        assert_eq!(
            why_the_window_stops(COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED),
            Some("WebView2's browser process ended")
        );
        assert_eq!(
            why_the_window_stops(COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED),
            Some("WebView2's render process ended")
        );
    }

    #[test]
    fn window_keeps_going_through_failures_webview2_recovers_from() {
        for kind in [
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE,
            COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_SANDBOX_HELPER_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_UNKNOWN_PROCESS_EXITED,
        ] {
            assert_eq!(why_the_window_stops(kind), None, "{kind:?}");
        }
    }

    #[test]
    fn notepad_is_windows_own() {
        let notepad = notepad().unwrap();
        assert!(notepad.ends_with(r"System32\notepad.exe"));
        assert!(notepad.is_absolute());
    }

    #[test]
    fn notepad_is_named_as_windows_names_it_not_by_its_file() {
        let name = notepad().and_then(|notepad| program_name(&notepad));
        assert!(
            name.as_deref()
                .is_some_and(|name| !name.is_empty() && !name.ends_with(".exe")),
            "{name:?}"
        );
    }

    #[test]
    fn missing_program_has_no_name() {
        assert_eq!(program_name(Path::new(r"C:\no such folder\none.exe")), None);
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
