//! The computer's clock, which names a pasted file.

use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

use crate::paste::LocalTime;

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
