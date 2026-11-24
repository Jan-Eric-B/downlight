//! Reading the clipboard and the local time for Paste (paste-markdown ticket 01). The clipboard is
//! opened and its text's memory locked through guards, so that it is always closed and unlocked
//! again, also on an early return or a panic: a clipboard left open blocks every other program's
//! copy and paste until this thread ends.

use std::ffi::c_void;
use std::marker::PhantomData;
use std::time::Duration;

use windows_sys::Win32::Foundation::{HANDLE, SYSTEMTIME};
use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

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
        let locked = LockedGlobal::lock(self, data)?;
        let text = locked
            .units()
            .split(|&unit| unit == 0)
            .next()
            .unwrap_or_default();
        Some(String::from_utf16_lossy(text))
    }
}

impl Drop for OpenedClipboard {
    fn drop(&mut self) {
        // SAFETY: matches the successful OpenClipboard that made `self`, on the same thread,
        // since `self` isn't `Send`; no handle from the clipboard outlives the borrow of `self`.
        unsafe { CloseClipboard() };
    }
}

/// A clipboard handle's global memory, locked until this is dropped.
struct LockedGlobal<'clipboard> {
    data: HANDLE,
    start: *mut c_void,
    /// The memory is the clipboard's, valid only while the clipboard stays open.
    clipboard: PhantomData<&'clipboard OpenedClipboard>,
}

impl<'clipboard> LockedGlobal<'clipboard> {
    /// Locks `data`, a handle from the open `_clipboard`, or None when its memory can't be
    /// locked.
    fn lock(_clipboard: &'clipboard OpenedClipboard, data: HANDLE) -> Option<Self> {
        // SAFETY: a CF_UNICODETEXT handle is a global memory object; a null pointer means the
        // memory couldn't be locked.
        let start = unsafe { GlobalLock(data) };
        (!start.is_null()).then_some(Self {
            data,
            start,
            clipboard: PhantomData,
        })
    }

    /// The memory as UTF-16 units, no further than the object's size: the text comes from
    /// another program, so it may lack the NUL that should end it.
    fn units(&self) -> &[u16] {
        // SAFETY: as for GlobalLock; a failure gives 0, which reads as no text.
        let size = unsafe { GlobalSize(self.data) };
        // SAFETY: `start` points to the `size` bytes of the locked object, which stays locked
        // while the slice borrows `self`. Global memory is 8-byte aligned, so UTF-16 units read
        // from it are aligned.
        unsafe { std::slice::from_raw_parts(self.start.cast::<u16>(), size / 2) }
    }
}

impl Drop for LockedGlobal<'_> {
    fn drop(&mut self) {
        // SAFETY: matches the successful GlobalLock that made `self`; no slice from `units`
        // outlives the borrow of `self`.
        unsafe { GlobalUnlock(self.data) };
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
