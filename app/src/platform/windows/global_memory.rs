//! Global memory locked for reading, as the clipboard's text and a drag's medium hand it over
//! (paste-markdown tickets 01 and 04). Both come from another program, so nothing is read past the
//! memory's size, and the guard unlocks it again also on an early return or a panic. And global
//! memory of this process's own, which the clipboard takes text in (copy-as-markdown ticket 01).

use std::ffi::c_void;
use std::marker::PhantomData;

use windows_sys::Win32::Foundation::{GlobalFree, HGLOBAL};
use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};

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

// SAFETY: the handle is only locked, read, written and freed, all of which any thread of the
// process may do: `holding_clipboard_text` writes it while it locks it, before it gives the
// handle to anyone, and only one owner holds the handle.
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

    /// New movable memory holding `text` as the clipboard takes its Unicode text
    /// ([`clipboard_units`]), encoded straight into the memory, so that a large text is never
    /// held in another form beside it (copy-as-markdown ticket 01). None when the system has no
    /// memory for it.
    pub fn holding_clipboard_text(text: &str) -> Option<Self> {
        let units = clipboard_units(text).count();
        let size = units.checked_mul(std::mem::size_of::<u16>())?;
        // SAFETY: GlobalAlloc takes no pointer; a null handle means it failed.
        let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, size) };
        if memory.is_null() {
            return None;
        }
        // SAFETY: the memory is new and nothing else has it.
        let owned = unsafe { Self::new(memory) };
        // SAFETY: as for `lock`.
        let start = unsafe { GlobalLock(owned.0) }.cast::<u16>();
        if start.is_null() {
            return None;
        }
        // Raw writes, since the memory isn't initialized and a reference to it would be.
        // `take(units)` ends the writes at the room the memory was asked for, whatever the
        // iterator would give.
        for (at, unit) in clipboard_units(text).take(units).enumerate() {
            // SAFETY: the locked memory has room for `units` UTF-16 units, as GlobalAlloc gives at
            // least as much as it is asked for, and `at` is below `units`. It is 8-byte aligned,
            // so aligned for UTF-16 units, and new, so nothing else reads or writes it.
            unsafe { start.add(at).write(unit) };
        }
        // SAFETY: matches the GlobalLock above; it answers 0 once the memory is unlocked for
        // good, which is what is meant.
        unsafe { GlobalUnlock(owned.0) };
        Some(owned)
    }

    /// Gives the memory up, to an owner that frees it, such as the clipboard once it takes it.
    pub fn into_handle(self) -> HGLOBAL {
        let memory = self.0;
        std::mem::forget(self);
        memory
    }

    /// The memory's handle, which stays this one's.
    pub fn handle(&self) -> HGLOBAL {
        self.0
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

/// `text` as `CF_UNICODETEXT` has it: UTF-16, each line break as CR LF, and the NUL that ends it.
/// A NUL in the text itself would end it early, so it is U+FFFD, as the render shows one.
fn clipboard_units(text: &str) -> impl Iterator<Item = u16> + '_ {
    text.chars()
        .flat_map(|character| {
            let (units, length) = match character {
                '\n' => ([u16::from(b'\r'), u16::from(b'\n')], 2),
                '\0' => ([0xFFFD, 0], 1),
                other => {
                    let mut units = [0; 2];
                    let length = other.encode_utf16(&mut units).len();
                    (units, length)
                }
            };
            units.into_iter().take(length)
        })
        .chain(std::iter::once(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a program reading `CF_UNICODETEXT` from memory holding `text` gets.
    fn read_back(text: &str) -> String {
        let memory = OwnedGlobal::holding_clipboard_text(text).unwrap();
        let locked = memory.lock().unwrap();
        String::from_utf16(locked.text()).unwrap()
    }

    #[test]
    fn clipboard_text_has_crlf_line_breaks_and_all_its_characters() {
        assert_eq!(read_back("# Größe\n\n😀 end"), "# Größe\r\n\r\n😀 end");
    }

    #[test]
    fn a_nul_in_clipboard_text_is_a_replacement_character_rather_than_its_end() {
        assert_eq!(read_back("a\0b\nc"), "a\u{FFFD}b\r\nc");
    }

    #[test]
    fn empty_clipboard_text_is_just_its_terminator() {
        let memory = OwnedGlobal::holding_clipboard_text("").unwrap();
        // Global memory may be rounded up, so only the text up to its NUL is the text.
        assert!(memory.lock().unwrap().text().is_empty());
    }

    #[test]
    fn clipboard_memory_is_the_size_of_the_text_in_utf16_and_its_terminator() {
        let memory = OwnedGlobal::holding_clipboard_text("a\n😀").unwrap();
        // a, CR, LF, a surrogate pair and the NUL: six units.
        let size = memory.lock().unwrap().bytes().len();
        assert!(size >= 12, "{size}");
        assert_eq!(clipboard_units("a\n😀").count(), 6);
    }
}
