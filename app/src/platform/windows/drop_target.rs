//! Downlight's own drop target (ADR 0007, paste-markdown tickets 03 and 04). Tauri's native drop
//! is off on Windows, and this takes its place on the webview's windows, as wry 0.57's
//! `DragDropController` does: it revokes WebView2's drop target, whose dropped files reach the
//! page without their paths, and registers one that reads the paths itself, and the text of a
//! drag without any, which wry's drop target refuses.
//!
//! All of it runs on the main thread: Tauri runs `with_webview`'s closure there, the webview's
//! windows that take drops belong to it, and OLE calls a drop target on its window's thread.

use std::cell::RefCell;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt as _;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;

use tauri::{Manager, Runtime, WebviewWindow, WebviewWindowBuilder};
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller4;
use windows::Win32::Foundation::{DRAGDROP_E_INVALIDHWND, HWND, LPARAM, LRESULT, POINTL, WPARAM};
use windows::Win32::System::Com::{
    CoTaskMemFree, DVASPECT_CONTENT, FORMATETC, IDataObject, STGMEDIUM, TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Ole::{
    CF_HDROP, CF_UNICODETEXT, CLIPBOARD_FORMAT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE,
    IDropTarget, IDropTarget_Impl, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Shell::{
    DefSubclassProc, DragQueryFileW, HDROP, IShellItemArray, RemoveWindowSubclass,
    SHCreateShellItemArrayFromDataObject, SIGDN_FILESYSPATH, SetWindowSubclass,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetWindowThreadProcessId, WM_DESTROY,
};
use windows_core::{BOOL, Interface as _, PCWSTR, Ref, implement, w};

use super::global_memory::LockedGlobal;
use crate::open::{DRAG_TAG, Dropped};

/// What a drop onto a window's page is handed to.
type OnDrop = Rc<dyn Fn(Dropped)>;

/// The clipboard format Chromium writes every custom `dataTransfer` type of a drag that starts in
/// a page into (`DataTransferCustomType` in Chromium's `clipboard_format_type_win.cc`): one format
/// for them all, holding a `base::Pickle` of the types and their data, rather than one format per
/// type. Registering a name answers the same format in every process.
const CHROMIUM_CUSTOM_DATA: PCWSTR = w!("Chromium Web Custom MIME Data Format");

/// The format [`CHROMIUM_CUSTOM_DATA`] is registered as, once for the process, since every drag
/// asks for it and the answer never changes; None should it fail to register, which only a full
/// atom table does.
static CHROMIUM_CUSTOM_DATA_FORMAT: OnceLock<Option<u16>> = OnceLock::new();

/// The id of the subclass that revokes the drop target as its window goes. One per window, so
/// any constant does.
const REVOKE_ON_DESTROY: usize = 1;

/// Turns Tauri's native drop off for a window, so that WebView2's own drop target is there for
/// [`add_drop_target`] to replace: wry's would refuse every drag without file paths.
pub fn with_drop_target<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    builder.disable_drag_drop_handler()
}

/// Calls `dropped`, on the main thread, with what is dropped onto `window`'s page: the paths of
/// the files of a drag with any, or else its plain text. A drag with paths, or with text that
/// isn't only whitespace, gets the copy cursor, never a move, so its source never deletes what it
/// dragged; any other gets the no-drop cursor, and so does a drag whose source allows no copy and
/// one that started in this window's page, which tags it (`open::DRAG_TAG`). The window shows no
/// highlight.
///
/// Each of the webview's windows gets a drop target of its own, which lives until that window is
/// destroyed: OLE holds the only reference to it from `RegisterDragDrop` until `RevokeDragDrop`
/// in the window's `WM_DESTROY`, and `dropped` goes with the last of them. OLE is initialized on
/// the main thread by tao, which does so for every window it creates with its own drag and drop
/// on, as Tauri leaves it (that is the window's, apart from the webview's handler).
///
/// Without the webview, which is only gone once the window is closing, there is nothing to drop
/// onto, so a failure to reach it is left at that.
pub fn add_drop_target(window: &WebviewWindow, dropped: impl Fn(Dropped) + Send + 'static) {
    let label = window.label().to_owned();
    let _ = window.with_webview(move |webview| {
        let controller = webview.controller();
        // As wry does: should WebView2's own drop target be left on a window, it then refuses a
        // drag from outside rather than handing it to the page.
        // SAFETY: plain calls on the live controller, on the window's thread.
        let _ = unsafe {
            controller
                .cast::<ICoreWebView2Controller4>()
                .and_then(|controller| controller.SetAllowExternalDrop(false))
        };
        let mut container = HWND::default();
        // SAFETY: `container` is a live value that ParentWindow only writes.
        if unsafe { controller.ParentWindow(&mut container) }.is_err() {
            return;
        }
        let dropped: OnDrop = Rc::new(dropped);
        let label: Rc<str> = Rc::from(label);
        for window in windows_of_this_thread_in(container) {
            register(window, &dropped, &label);
        }
    });
}

/// The windows inside `container`, at any depth, that belong to this thread. WebView2's windows
/// of its own processes are among them, and OLE takes no drop target for those, nor Windows a
/// subclass.
fn windows_of_this_thread_in(container: HWND) -> Vec<HWND> {
    unsafe extern "system" fn collect(window: HWND, windows: LPARAM) -> BOOL {
        // SAFETY: `windows` is the vector EnumChildWindows was given below, which lives through
        // the call and is borrowed nowhere else meanwhile.
        unsafe { &mut *(windows.0 as *mut Vec<HWND>) }.push(window);
        true.into()
    }
    let mut windows: Vec<HWND> = Vec::new();
    // SAFETY: `collect` only pushes onto the vector passed as its LPARAM, which outlives the call.
    let _ = unsafe {
        EnumChildWindows(
            Some(container),
            Some(collect),
            LPARAM(std::ptr::from_mut(&mut windows) as isize),
        )
    };
    // SAFETY: a plain call.
    let thread = unsafe { GetCurrentThreadId() };
    windows.retain(|&window| {
        // SAFETY: a plain call; a window that is gone meanwhile answers thread 0, which is no
        // thread.
        unsafe { GetWindowThreadProcessId(window, None) == thread }
    });
    windows
}

/// Puts a drop target of Downlight's own on `window` in place of WebView2's, revoked again as the
/// window goes. The subclass comes first and goes again should registering fail, so a window
/// never keeps a target that nothing revokes.
fn register(window: HWND, dropped: &OnDrop, label: &Rc<str>) {
    // SAFETY: `window` belongs to this thread, as Windows requires of a subclass, and the
    // procedure lives as long as the process.
    if !unsafe { SetWindowSubclass(window, Some(revoke_on_destroy), REVOKE_ON_DESTROY, 0) }
        .as_bool()
    {
        return;
    }
    let target: IDropTarget = DropTarget {
        dropped: Rc::clone(dropped),
        label: Rc::clone(label),
        drag: RefCell::new(None),
    }
    .into();
    // SAFETY: plain calls on a window of this thread, on which tao has initialized OLE.
    let replaced = unsafe {
        // Only a window that is gone meanwhile (DRAGDROP_E_INVALIDHWND) gets no target. Any other
        // answer still registers one, as wry does: a window WebView2 registered no target on
        // answers DRAGDROP_E_NOTREGISTERED, and needs one too.
        let gone =
            matches!(RevokeDragDrop(window), Err(error) if error.code() == DRAGDROP_E_INVALIDHWND);
        !gone && RegisterDragDrop(window, &target).is_ok()
    };
    if !replaced {
        // SAFETY: the subclass set above, on this thread.
        let _ = unsafe { RemoveWindowSubclass(window, Some(revoke_on_destroy), REVOKE_ON_DESTROY) };
    }
}

/// Lets go of a window's drop target while the window is still there to revoke it from:
/// `RevokeDragDrop` releases OLE's reference, the only one, and with it the target.
unsafe extern "system" fn revoke_on_destroy(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if message == WM_DESTROY {
        // SAFETY: plain calls on the window being destroyed, which is still valid during
        // WM_DESTROY, on its thread. Should WebView2 have revoked the target first, the revoke
        // answers that none is registered.
        unsafe {
            let _ = RevokeDragDrop(window);
            let _ = RemoveWindowSubclass(window, Some(revoke_on_destroy), REVOKE_ON_DESTROY);
        }
    }
    // SAFETY: the arguments are the message's own.
    unsafe { DefSubclassProc(window, message, wparam, lparam) }
}

/// The drop target on one of a webview's windows. Not agile: it holds `Rc`s and cells, and OLE
/// calls it only on its window's thread, the main thread.
#[implement(IDropTarget, Agile = false)]
struct DropTarget {
    dropped: OnDrop,
    /// The label of the window whose page this is, which refuses the drags its page tagged.
    label: Rc<str>,
    /// What the drag over the window carries, kept from `DragEnter` until `DragLeave` or `Drop`
    /// when the window takes it. A text drag's medium is the target's from `GetData` until it is
    /// released, so `Drop` reads the text from it instead of having the source copy all of it
    /// across processes again, on the main thread.
    drag: RefCell<Option<Drag>>,
}

impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(
        &self,
        data: Ref<'_, IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        // A drag the window can't copy is never taken, so its data isn't read at all.
        let drag = if allows_copy(effect) {
            Drag::onto(data.as_ref(), &self.label)
        } else {
            None
        };
        answer(effect, drag.is_some());
        // Replacing a drag an earlier DragEnter kept releases its medium.
        self.drag.replace(drag);
        Ok(())
    }

    fn DragOver(
        &self,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        // The drag's data stays the same until it leaves, so DragEnter's decision holds.
        answer(effect, self.drag.borrow().is_some());
        Ok(())
    }

    fn DragLeave(&self) -> windows_core::Result<()> {
        drop(self.drag.take());
        Ok(())
    }

    fn Drop(
        &self,
        _data: Ref<'_, IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        // Drop is given the data object DragEnter was given, so the drag kept there is the one
        // dropped: its text is copied out of the kept medium before this returns, and the medium
        // released after. The effect tells the source what became of its data: left unset it would still
        // hold every effect the source allows, a move among them.
        let drag = self.drag.take();
        let copies = answer(effect, drag.is_some());
        if let Some(dropped) = drag.filter(|_| copies).and_then(Drag::take) {
            (self.dropped)(dropped);
        }
        Ok(())
    }
}

/// Whether the drag's source allows a copy, as `effect` holds the effects it allows on entry to
/// each call (`DoDragDrop`'s `dwOKEffects`).
fn allows_copy(effect: *const DROPEFFECT) -> bool {
    // SAFETY: OLE passes a pointer to its own effect for each call, valid through it; a null one
    // allows nothing.
    unsafe { effect.as_ref() }.is_some_and(|allowed| allowed.contains(DROPEFFECT_COPY))
}

/// Answers OLE with the effect of a drag, and gives whether it is a copy. OLE's contract is to
/// choose one of the effects the source allows, or none: a copy when the window `takes` the drag
/// and the source allows one, so the source never deletes what it dragged, and none otherwise,
/// never a move.
fn answer(effect: *mut DROPEFFECT, takes: bool) -> bool {
    let copies = takes && allows_copy(effect);
    // SAFETY: as for `allows_copy`; a null pointer is skipped.
    if let Some(effect) = unsafe { effect.as_mut() } {
        *effect = if copies {
            DROPEFFECT_COPY
        } else {
            DROPEFFECT_NONE
        };
    }
    copies
}

/// What a drag onto a window carries that the window takes.
enum Drag {
    Files(Vec<PathBuf>),
    /// Text that isn't only whitespace, in the medium `GetData` gave, which is the target's until
    /// it is released.
    Text(Medium),
}

impl Drag {
    /// What `data`, dragged onto the window labelled `label`, carries, or None for a drag the
    /// window refuses: one that started in its own page, or one with neither paths nor text to
    /// show. Paths come first, so a drag with any is a file drop, as before text drops.
    fn onto(data: Option<&IDataObject>, label: &str) -> Option<Self> {
        let data = data?;
        if started_in(data, label) {
            return None;
        }
        let paths = dragged_paths(data);
        if !paths.is_empty() {
            return Some(Self::Files(paths));
        }
        let text = Medium::get(data, CF_UNICODETEXT)?;
        let shows = text
            .lock()
            .is_some_and(|locked| shows_something(locked.text()));
        shows.then_some(Self::Text(text))
    }

    /// What is dropped, its text copied out of the drag's medium.
    fn take(self) -> Option<Dropped> {
        match self {
            Self::Files(paths) => Some(Dropped::Files(paths)),
            Self::Text(medium) => Some(Dropped::Text(String::from_utf16_lossy(
                medium.lock()?.text(),
            ))),
        }
    }
}

/// Whether `text` holds more than whitespace, as Paste's check of the text it writes tells it
/// (`str::trim`), without copying it: a unit that isn't a character counts as one, as it is
/// copied out as U+FFFD.
fn shows_something(text: &[u16]) -> bool {
    char::decode_utf16(text.iter().copied()).any(|unit| !unit.is_ok_and(char::is_whitespace))
}

/// Whether the drag started in the page of the window labelled `label`, which tags each of its
/// drags with its label (`open::DRAG_TAG`). The tag comes as the page's `dataTransfer` type, in
/// Chromium's custom data; a drag without it, or with one that can't be read, counts as from
/// elsewhere, so a tag never refuses a drag from another program.
fn started_in(data: &IDataObject, label: &str) -> bool {
    let format = CHROMIUM_CUSTOM_DATA_FORMAT.get_or_init(|| {
        // SAFETY: a plain call with a NUL-terminated literal.
        let format = unsafe { RegisterClipboardFormatW(CHROMIUM_CUSTOM_DATA) };
        // A registered format is between 0xC000 and 0xFFFF; 0 means it couldn't be registered.
        u16::try_from(format).ok().filter(|&format| format != 0)
    });
    let Some(format) = *format else {
        return false;
    };
    Medium::get(data, CLIPBOARD_FORMAT(format)).is_some_and(|medium| {
        medium
            .lock()
            .is_some_and(|locked| is_tagged_by(locked.bytes(), label))
    })
}

/// Whether Chromium's custom data `pickle` holds the tag of the window labelled `label`.
fn is_tagged_by(pickle: &[u8], label: &str) -> bool {
    custom_data(pickle, DRAG_TAG).is_some_and(|tag| tag.eq(label.encode_utf16()))
}

/// The data of the custom `dataTransfer` type `wanted` in Chromium's custom data, as UTF-16
/// units, or None when it isn't there or the data doesn't read as Chromium reads it: it comes
/// from another program.
///
/// Chromium writes the types and their data into a `base::Pickle` (`WriteCustomDataToPickle` in
/// `custom_data_helper.cc`, `base/pickle.cc`): a header holding the payload's size in bytes,
/// then, in the payload, the number of types, and each type and its data as a string: its length
/// in UTF-16 units and the units, padded to 4 bytes. Every number is 32 bits, little-endian.
fn custom_data<'pickle>(
    pickle: &'pickle [u8],
    wanted: &str,
) -> Option<impl Iterator<Item = u16> + use<'pickle>> {
    let mut header = Pickle(pickle);
    let payload_size = usize::try_from(header.u32()?).ok()?;
    let mut payload = Pickle(header.0.get(..payload_size)?);
    for _ in 0..payload.u32()? {
        let custom_type = payload.string()?;
        let data = payload.string()?;
        if units(custom_type).eq(wanted.encode_utf16()) {
            return Some(units(data));
        }
    }
    None
}

/// A string's UTF-16 units, as a pickle holds them.
fn units(string: &[u8]) -> impl Iterator<Item = u16> + '_ {
    string
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&unit| u16::from_le_bytes(unit))
}

/// What is left to read of a `base::Pickle`. Each read takes from its start, or gives None when
/// not enough is left.
struct Pickle<'pickle>(&'pickle [u8]);

impl<'pickle> Pickle<'pickle> {
    fn u32(&mut self) -> Option<u32> {
        let (number, rest) = self.0.split_first_chunk::<4>()?;
        self.0 = rest;
        Some(u32::from_le_bytes(*number))
    }

    /// A string's UTF-16 units as bytes. Its length is a signed number, which a negative one
    /// fails. A last string whose padding is cut off still reads, as Chromium's own reader
    /// (`AlignAfterRead` in `base/pickle.cc`) skips only the padding there is.
    fn string(&mut self) -> Option<&'pickle [u8]> {
        let length = usize::try_from(self.u32()?.cast_signed()).ok()?;
        let bytes = length.checked_mul(2)?;
        let padded = bytes.checked_next_multiple_of(4)?;
        let string = self.0.get(..bytes)?;
        self.0 = self.0.get(padded..).unwrap_or_default();
        Some(string)
    }
}

/// The paths of the files a drag carries, as wry 0.57 reads them, or none for a drag of
/// anything else. Explorer can't put a path of 260 characters or longer into `CF_HDROP` (it
/// fails with `ERROR_INSUFFICIENT_BUFFER`, long paths enabled or not), so the shell item array,
/// which has no such limit, is read when it fails.
fn dragged_paths(data: &IDataObject) -> Vec<PathBuf> {
    match Medium::get(data, CF_HDROP) {
        Some(medium) => medium.dropped_files(),
        None => shell_item_paths(data),
    }
}

/// The file system paths of the shell items a drag carries. An item with none, such as a virtual
/// folder's, is skipped.
fn shell_item_paths(data: &IDataObject) -> Vec<PathBuf> {
    // SAFETY: a plain call on the drag's live data object.
    let Ok(items) = (unsafe { SHCreateShellItemArrayFromDataObject::<_, IShellItemArray>(data) })
    else {
        return Vec::new();
    };
    // SAFETY: plain calls on the live array.
    let count = unsafe { items.GetCount() }.unwrap_or(0);
    (0..count)
        .filter_map(|index| {
            // SAFETY: plain calls on the live array and its item. The name is the item's string,
            // NUL-terminated, which is copied before CoTaskMemFree frees it, once.
            unsafe {
                let item = items.GetItemAt(index).ok()?;
                let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
                let path = PathBuf::from(OsString::from_wide(name.as_wide()));
                CoTaskMemFree(Some(name.0.cast()));
                Some(path)
            }
        })
        .collect()
}

/// A drag's data in one format, in global memory, released when this is dropped, also on an early
/// return or a panic.
struct Medium(STGMEDIUM);

impl Medium {
    /// The drag's data in `format`, or None when it has none in global memory. The drag comes
    /// from another program, so a medium of another kind than asked for is released untouched.
    fn get(data: &IDataObject, format: CLIPBOARD_FORMAT) -> Option<Self> {
        let asked = FORMATETC {
            cfFormat: format.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        // SAFETY: `asked` is valid for the call, and the medium it answers is released below.
        let medium = Self(unsafe { data.GetData(&asked) }.ok()?);
        (medium.0.tymed == TYMED_HGLOBAL.0 as u32).then_some(medium)
    }

    /// The paths in a `CF_HDROP` medium, each as long as it is: Windows takes paths longer than
    /// 260 characters in some cases.
    fn dropped_files(&self) -> Vec<PathBuf> {
        // SAFETY: a medium asked for as TYMED_HGLOBAL, and checked to be one, holds its global
        // memory in `hGlobal`; in CF_HDROP that is the HDROP.
        let files = HDROP(unsafe { self.0.u.hGlobal }.0);
        // SAFETY: plain calls on the live HDROP; u32::MAX asks for the number of files, and a null
        // buffer for a path's length without its NUL. Each buffer has room for its path and NUL,
        // and only as much as was copied is read.
        unsafe {
            (0..DragQueryFileW(files, u32::MAX, None))
                .map(|index| {
                    let length = DragQueryFileW(files, index, None) as usize;
                    let mut path = vec![0; length + 1];
                    let copied = DragQueryFileW(files, index, Some(&mut path)) as usize;
                    PathBuf::from(OsString::from_wide(&path[..copied.min(length)]))
                })
                .collect()
        }
    }

    /// The medium's global memory, locked, or None when it can't be locked.
    fn lock(&self) -> Option<LockedGlobal<'_>> {
        // SAFETY: a medium asked for as TYMED_HGLOBAL, and checked to be one, holds its global
        // memory in `hGlobal`, which stays allocated until the medium is released, and the guard
        // borrows the medium, which releases it.
        unsafe { LockedGlobal::lock(self, self.0.u.hGlobal.0) }
    }
}

impl Drop for Medium {
    fn drop(&mut self) {
        // SAFETY: the medium GetData gave, released once, here.
        unsafe { ReleaseStgMedium(&mut self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `base::Pickle`'s payload, written number by number as Chromium's
    /// `WriteCustomDataToPickle` writes it, or as a program that only looks like Chromium might.
    #[derive(Default)]
    struct Payload(Vec<u8>);

    impl Payload {
        fn number(mut self, number: u32) -> Self {
            self.0.extend(number.to_le_bytes());
            self
        }

        /// A string's length in UTF-16 units, which a pickle writes as a signed number.
        fn length(self, length: i32) -> Self {
            self.number(length.cast_unsigned())
        }

        /// A string's length and units, without the padding after them.
        fn unpadded(self, string: &str) -> Self {
            let units: Vec<u16> = string.encode_utf16().collect();
            let mut payload = self.length(units.len() as i32);
            payload
                .0
                .extend(units.iter().flat_map(|unit| unit.to_le_bytes()));
            payload
        }

        /// A string as Chromium writes it: its length, its units and the padding to 4 bytes.
        fn string(self, string: &str) -> Self {
            let mut payload = self.unpadded(string);
            payload.0.resize(payload.0.len().next_multiple_of(4), 0);
            payload
        }

        /// The pickle: the header, holding the payload's size in bytes, then the payload.
        fn pickle(self) -> Vec<u8> {
            let mut pickle = (self.0.len() as u32).to_le_bytes().to_vec();
            pickle.extend(self.0);
            pickle
        }
    }

    /// Chromium's custom data of a drag whose page set `types`, each with its data.
    fn custom(types: &[(&str, &str)]) -> Vec<u8> {
        types
            .iter()
            .fold(
                Payload::default().number(types.len() as u32),
                |payload, (kind, data)| payload.string(kind).string(data),
            )
            .pickle()
    }

    fn found(pickle: &[u8], wanted: &str) -> Option<String> {
        custom_data(pickle, wanted)
            .map(|units| String::from_utf16_lossy(&units.collect::<Vec<_>>()))
    }

    #[test]
    fn custom_data_finds_the_tag_s_data_among_other_types() {
        // "text/x" is 6 units, 12 bytes, and "abc" 3 units, 6 bytes padded to 8, so the tag is
        // read past padding.
        let pickle = custom(&[("text/x", "abc"), (DRAG_TAG, "main-2")]);
        assert_eq!(found(&pickle, DRAG_TAG).as_deref(), Some("main-2"));
        assert!(is_tagged_by(&pickle, "main-2"));
    }

    #[test]
    fn custom_data_without_the_type_is_none() {
        let pickle = custom(&[("text/x", "abc")]);
        assert_eq!(found(&pickle, DRAG_TAG), None);
        assert!(!is_tagged_by(&pickle, "main"));
    }

    #[test]
    fn a_tag_with_another_window_s_label_is_not_this_window_s() {
        let pickle = custom(&[(DRAG_TAG, "window-2")]);
        assert!(!is_tagged_by(&pickle, "main"));
        assert!(is_tagged_by(&pickle, "window-2"));
    }

    #[test]
    fn an_empty_buffer_or_a_header_shorter_than_4_bytes_is_none() {
        assert_eq!(found(&[], DRAG_TAG), None);
        assert_eq!(found(&[4, 0, 0], DRAG_TAG), None);
    }

    #[test]
    fn a_payload_size_past_the_end_is_none() {
        let mut pickle = custom(&[(DRAG_TAG, "main")]);
        let size = u32::from_le_bytes([pickle[0], pickle[1], pickle[2], pickle[3]]);
        pickle[..4].copy_from_slice(&(size + 4).to_le_bytes());
        assert_eq!(found(&pickle, DRAG_TAG), None);
    }

    #[test]
    fn a_negative_string_length_is_none() {
        let pickle = Payload::default().number(1).length(-1).pickle();
        assert_eq!(found(&pickle, DRAG_TAG), None);
    }

    #[test]
    fn a_string_length_of_0x7fff_ffff_is_none() {
        let pickle = Payload::default()
            .number(1)
            .length(0x7FFF_FFFF)
            .number(0)
            .pickle();
        assert_eq!(found(&pickle, DRAG_TAG), None);
    }

    #[test]
    fn the_last_string_without_its_padding_still_reads_as_chromium_reads_it() {
        let pickle = Payload::default()
            .number(1)
            .string(DRAG_TAG)
            .unpadded("abc")
            .pickle();
        assert_eq!(found(&pickle, DRAG_TAG).as_deref(), Some("abc"));
    }

    #[test]
    fn a_count_larger_than_the_strings_present_is_none() {
        let pickle = Payload::default()
            .number(3)
            .string("text/x")
            .string("abc")
            .pickle();
        assert_eq!(found(&pickle, DRAG_TAG), None);
    }

    #[test]
    fn shows_something_is_false_for_nothing_but_whitespace() {
        for text in ["", " \r\n\t\n  ", "\u{3000}\u{a0}"] {
            let units: Vec<u16> = text.encode_utf16().collect();
            assert!(!shows_something(&units), "{text:?}");
            assert!(text.trim().is_empty(), "{text:?}");
        }
    }

    #[test]
    fn shows_something_is_true_for_visible_text_and_a_lone_surrogate_as_paste_tells_it() {
        let visible: Vec<u16> = "  # Notes\n".encode_utf16().collect();
        let lone_surrogate = [u16::from(b' '), 0xD800, u16::from(b' ')];
        for units in [visible.as_slice(), &lone_surrogate] {
            assert!(shows_something(units), "{units:?}");
            // Paste checks the text it is given, copied out with U+FFFD for the surrogate.
            assert!(
                !String::from_utf16_lossy(units).trim().is_empty(),
                "{units:?}"
            );
        }
    }
}
