//! Global memory locked for reading, as the clipboard's text and a drag's medium hand it over
//! (paste-markdown tickets 01 and 04). Both come from another program, so nothing is read past the
//! memory's size, and the guard unlocks it again also on an early return or a panic.

use std::ffi::c_void;
use std::marker::PhantomData;

use windows_sys::Win32::Foundation::{GlobalFree, HGLOBAL};
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

    /// The memory as `CF_UNICODETEXT`'s UTF-16 text, up to its first NUL, which the format says
    /// ends the text, and never past the memory's size.
    ///
    /// The size is no length for the text: GlobalSize may answer more than was asked for when the
    /// memory was allocated, and nothing says what the bytes past that hold. So a program that
    /// leaves the NUL out may get a few stray characters after its text, as many as the
    /// allocator rounded up by. That is still taken rather than refused: the bytes are the
    /// memory's own, so reading them is safe, the text is shown as a document of its own and
    /// never trusted, and refusing it would lose all of the reader's text for a missing
    /// terminator.
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

/// Global memory that is this process's own, freed when this is dropped. It can go to another
/// thread, unlike the medium it came from, so that copying a large drop's text out of it holds
/// up no window (paste-markdown ticket 04): a global memory handle is the process's, valid on any
/// of its threads.
pub struct OwnedGlobal(HGLOBAL);

// SAFETY: the handle is only locked, read and freed, all of which any thread of the process may
// do, and only one owner holds it.
unsafe impl Send for OwnedGlobal {}

impl OwnedGlobal {
    /// Takes `memory` over.
    ///
    /// # Safety
    ///
    /// `memory` is a global memory handle that nothing else frees or uses once this has it.
    pub unsafe fn new(memory: HGLOBAL) -> Self {
        Self(memory)
    }

    /// The memory, locked, or None when it can't be locked.
    pub fn lock(&self) -> Option<LockedGlobal<'_>> {
        // SAFETY: the memory is this one's, allocated until it is dropped, and the guard borrows
        // it.
        unsafe { LockedGlobal::lock(self, self.0) }
    }
}

impl Drop for OwnedGlobal {
    fn drop(&mut self) {
        // SAFETY: the memory is this one's, freed once, here, after every guard borrowing it has
        // unlocked it.
        unsafe { GlobalFree(self.0) };
    }
}
