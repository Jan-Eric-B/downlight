//! Global memory locked for reading, as the clipboard's text and a drag's medium hand it over
//! (paste-markdown tickets 01 and 04). Both come from another program, so nothing in the memory
//! says where it ends but its size, and the guard unlocks it again also on an early return or a
//! panic.

use std::ffi::c_void;
use std::marker::PhantomData;

use windows_sys::Win32::Foundation::HGLOBAL;
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

/// Global memory, locked until this is dropped. The memory is its owner's (the open clipboard, a
/// drag's medium), valid only while the owner is, so this borrows the owner.
pub struct LockedGlobal<'owner> {
    memory: HGLOBAL,
    start: *const c_void,
    size: usize,
    owner: PhantomData<&'owner ()>,
}

impl<'owner> LockedGlobal<'owner> {
    /// Locks `memory`, which `_owner` holds, or None when it can't be locked.
    ///
    /// # Safety
    ///
    /// `memory` is a global memory handle that stays allocated while `_owner` is borrowed.
    pub unsafe fn lock<Owner: ?Sized>(_owner: &'owner Owner, memory: HGLOBAL) -> Option<Self> {
        // SAFETY: the caller's promise; a null pointer means the memory couldn't be locked.
        let start = unsafe { GlobalLock(memory) };
        if start.is_null() {
            return None;
        }
        // SAFETY: as for GlobalLock; a failure gives 0, which reads as empty.
        let size = unsafe { GlobalSize(memory) };
        Some(Self {
            memory,
            start,
            size,
            owner: PhantomData,
        })
    }

    /// The memory, no further than its size.
    pub fn bytes(&self) -> &[u8] {
        // SAFETY: `start` points to the `size` bytes of the locked memory, which stays locked
        // while the slice borrows `self`.
        unsafe { std::slice::from_raw_parts(self.start.cast::<u8>(), self.size) }
    }

    /// The memory as `CF_UNICODETEXT`'s UTF-16 text, up to its first NUL and never past its size,
    /// since the program the text comes from may leave the NUL out.
    pub fn text(&self) -> &[u16] {
        // SAFETY: as for `bytes`, in whole units. Global memory is 8-byte aligned, so UTF-16
        // units read from it are aligned.
        let units = unsafe { std::slice::from_raw_parts(self.start.cast::<u16>(), self.size / 2) };
        units.split(|&unit| unit == 0).next().unwrap_or_default()
    }
}

impl Drop for LockedGlobal<'_> {
    fn drop(&mut self) {
        // SAFETY: matches the successful GlobalLock that made `self`; no slice from it outlives
        // the borrow of `self`. It answers 0 once the memory is unlocked for good, which is what
        // is meant.
        unsafe { GlobalUnlock(self.memory) };
    }
}
