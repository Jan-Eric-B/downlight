//! Reading the clipboard and the local time for Paste (paste-markdown ticket 01). The clipboard is
//! opened and its text's memory locked through guards, so that it is always closed and unlocked
//! again, also on an early return or a panic: a clipboard left open blocks every other program's
//! copy and paste until this thread ends.

use std::marker::PhantomData;
use std::time::Duration;

use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

use super::global_memory::LockedGlobal;
use crate::paste::{Clipboard, LocalTime};

/// How many times Paste tries to open the clipboard while another program has it open, and how
/// long it waits between tries. Programs hold it open only for a moment while they copy or read,
/// so a quarter of a second rides that out, and is still too short for the reader to wait on.
const CLIPBOARD_TRIES: u32 = 10;
const CLIPBOARD_RETRY_WAIT: Duration = Duration::from_millis(25);

/// The clipboard's plain text, for Paste: Windows' Unicode text, which it also gives for text
/// copied only in the ANSI or OEM formats. No text, as for an image or copied files, and a
/// clipboard another program keeps open through every try, say so.
pub fn clipboard_text() -> Clipboard {
    let mut tries = 1;
    let open = loop {
        if let Some(open) = OpenedClipboard::open() {
            break open;
        }
        if tries == CLIPBOARD_TRIES {
            return Clipboard::Busy;
        }
        tries += 1;
        std::thread::sleep(CLIPBOARD_RETRY_WAIT);
    };
    open.text().map_or(Clipboard::NoText, Clipboard::Text)
}

/// The clipboard, open on this thread until this is dropped. Not `Send`: the clipboard is open
/// for the thread that opened it, which must close it.
struct OpenedClipboard {
    on_this_thread: PhantomData<*const ()>,
}

impl OpenedClipboard {
    /// Opens the clipboard, or None while another program has it open.
    fn open() -> Option<Self> {
        // SAFETY: OpenClipboard takes no pointer of ours; with no window, the clipboard is opened
        // for this thread, which the guard closes it on.
        let opened = unsafe { OpenClipboard(std::ptr::null_mut()) } != 0;
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
}

impl Drop for OpenedClipboard {
    fn drop(&mut self) {
        // SAFETY: matches the successful OpenClipboard that made `self`, on the same thread,
        // since `self` isn't `Send`; no handle from the clipboard outlives the borrow of `self`.
        unsafe { CloseClipboard() };
    }
}

/// The time now on the computer's clock, which names a pasted file.
pub fn local_time() -> LocalTime {
    let mut now = SYSTEMTIME::default();
    // SAFETY: `now` is a live SYSTEMTIME that GetLocalTime only writes.
    unsafe { GetLocalTime(&mut now) };
    LocalTime {
        year: now.wYear,
        month: now.wMonth,
        day: now.wDay,
        hour: now.wHour,
        minute: now.wMinute,
        second: now.wSecond,
    }
}
