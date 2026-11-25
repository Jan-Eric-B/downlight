//! Reading the clipboard for Paste (paste-markdown ticket 01), and writing it for Copy as
//! Markdown (copy-as-markdown ticket 01). The clipboard is opened and its text's memory locked
//! through guards, so that it is always closed and unlocked again, also on an early return or a
//! panic: a clipboard left open blocks every other program's copy and paste until this thread
//! ends.

use std::io;
use std::marker::PhantomData;
use std::time::Duration;

use tauri::WebviewWindow;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

use super::global_memory::{LockedGlobal, OwnedGlobal};
use crate::copy::ClipboardError;
use crate::paste::Clipboard;

/// How many times Paste and Copy as Markdown try to open the clipboard while another program has
/// it open, and how long they wait between tries. Programs hold it open only for a moment while
/// they copy or read, so a quarter of a second rides that out, and is still too short for the
/// reader to wait on.
const CLIPBOARD_TRIES: u32 = 10;
const CLIPBOARD_RETRY_WAIT: Duration = Duration::from_millis(25);

/// The clipboard's plain text, for Paste: Windows' Unicode text, which it also gives for text
/// copied only in the ANSI or OEM formats. No text, as for an image or copied files, and a
/// clipboard another program keeps open through every try, say so.
pub fn clipboard_text() -> Clipboard {
    let Some(open) = OpenedClipboard::open_trying(std::ptr::null_mut()) else {
        return Clipboard::Busy;
    };
    open.text().map_or(Clipboard::NoText, Clipboard::Text)
}

/// Puts `text` on the clipboard as Windows' Unicode text, in place of what it held, for Copy as
/// Markdown, with its line breaks as `\r\n`, as Windows' text has them. Windows makes the ANSI
/// and OEM formats from it for a program that asks for those. The clipboard is opened for
/// `window`, which then owns it: opened for no window, Windows gives the clipboard no owner, and
/// says that setting its data then fails.
pub fn write_clipboard_text(window: &WebviewWindow, text: &str) -> Result<(), ClipboardError> {
    let owner = window.hwnd().map_err(|error| {
        ClipboardError::Failed(io::Error::other(format!("the window is gone: {error}")))
    })?;
    // Made before the clipboard is opened, so that it is held open no longer than it takes to
    // hand the memory over.
    let memory = OwnedGlobal::holding_clipboard_text(text)
        .ok_or_else(|| ClipboardError::Failed(io::Error::from(io::ErrorKind::OutOfMemory)))?;
    let open = OpenedClipboard::open_trying(owner.0).ok_or(ClipboardError::Busy)?;
    open.replace_text(memory).map_err(ClipboardError::Failed)
}

/// The clipboard, open on this thread until this is dropped. Not `Send`: the clipboard is open
/// for the thread that opened it, which must close it.
struct OpenedClipboard {
    on_this_thread: PhantomData<*const ()>,
}

impl OpenedClipboard {
    /// Opens the clipboard for `owner`, or for this thread when it is null, trying again a few
    /// times while another program has it open; None when it had it open through every try.
    fn open_trying(owner: HWND) -> Option<Self> {
        let mut tries = 1;
        loop {
            if let Some(open) = Self::open(owner) {
                return Some(open);
            }
            if tries == CLIPBOARD_TRIES {
                return None;
            }
            tries += 1;
            std::thread::sleep(CLIPBOARD_RETRY_WAIT);
        }
    }

    /// Opens the clipboard for `owner`, or None while another program has it open.
    fn open(owner: HWND) -> Option<Self> {
        // SAFETY: OpenClipboard takes no pointer of ours, and a window that has closed meanwhile
        // only makes it fail. The clipboard is opened on this thread, which the guard closes it
        // on.
        let opened = unsafe { OpenClipboard(owner) } != 0;
        opened.then_some(Self {
            on_this_thread: PhantomData,
        })
    }

    /// The text on the clipboard, or None when it holds no text.
    fn text(&self) -> Option<String> {
        // SAFETY: the clipboard is open on this thread while `self` lives. The handle stays the
        // clipboard's: it is only read here, never freed, and unlocked before `self` closes it.
        let data = unsafe { GetClipboardData(u32::from(CF_UNICODETEXT)) };
        if data.is_null() {
            return None;
        }
        // SAFETY: a CF_UNICODETEXT handle is a global memory object, which stays the clipboard's
        // while it is open, and the guard borrows `self`, which closes it.
        let locked = unsafe { LockedGlobal::lock(self, data) }?;
        Some(String::from_utf16_lossy(locked.text()))
    }

    /// Empties the clipboard and gives it `text`, NUL-terminated UTF-16 in global memory. The
    /// clipboard owns the memory once it took it; memory it refused is freed.
    fn replace_text(&self, text: OwnedGlobal) -> io::Result<()> {
        // SAFETY: the clipboard is open on this thread while `self` lives.
        if unsafe { EmptyClipboard() } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the clipboard is open on this thread and emptied for its owner, and the handle
        // is movable global memory holding NUL-terminated UTF-16, as CF_UNICODETEXT takes it.
        let taken = unsafe { SetClipboardData(u32::from(CF_UNICODETEXT), text.handle()) };
        if taken.is_null() {
            // Taken before `text` is dropped, which frees the memory and may set another error.
            return Err(io::Error::last_os_error());
        }
        // The clipboard frees it when it is emptied next.
        let _clipboards = text.into_handle();
        Ok(())
    }
}

impl Drop for OpenedClipboard {
    fn drop(&mut self) {
        // SAFETY: matches the successful OpenClipboard that made `self`, on the same thread,
        // since `self` isn't `Send`; no handle from the clipboard outlives the borrow of `self`.
        unsafe { CloseClipboard() };
    }
}
