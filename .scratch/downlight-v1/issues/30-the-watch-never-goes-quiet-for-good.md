# 30: A document's watch never goes quiet for good

**What to build:** A window notices every change to its document's file, even when the file system watch stops or misses events, and the reader is told when a file can't be watched. Today the watch can stop without a word, and then saves never reload the document again.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] Research first, recorded in this ticket: what notify 8.2.2 does on Windows when the watched folder is deleted, moved or recycled; when `ReadDirectoryChangesW` completes with an error (for example a share's session dropped after sleep or a VPN reconnect); when its buffer overflows; and when re-arming fails. Read from notify's source and Microsoft Learn; check the cases that need a real file system in the release build.
- [ ] The window checks the file's stamp again when it gets the focus, and reloads (or shows the bar) when it differs, whatever the watch said. This catches every missed event at the moment the reader looks.
- [ ] After `Gone`, the watch keeps looking for the document's folder and file, and reports `Back` when the file returns, even if its folder was deleted and recreated in between (a build that wipes its output folder, a branch switch). Story 40.
- [ ] A watch that stops (notify unwatches the folder) is noticed and restarted, or the reader is told that only F5 reloads the document.
- [ ] A save made between the render ahead's read and the start of the watch reloads the document. Today the stamp is taken after `watcher.watch`, so a save before the watch gives no event and a save between the watch and the stamp matches the stamp.
- [ ] A folder over 260 characters is watched, or the reader is told why not (the manifest has no `longPathAware`).
- [ ] Checked by hand: renaming or deleting the document's folder, or a parent, in Explorer while the window is open. If the open directory handle blocks it ("folder in use"), find a way that doesn't, or record why it must.
- [ ] Tested first where a test can observe it: in `app/src/watch.rs` with real temporary folders, a folder deleted and recreated, renamed away and back, and a save while the watch starts.

## Comments

**2026-11-13, from the architecture review.** `app/src/watch.rs` counts a watcher error as a touch (`event.as_ref().map_or(true, ...)`), but notify 8.2.2 never calls the handler with one in these cases. In `src/windows.rs`: on `ERROR_ACCESS_DENIED` with the folder gone it unwatches silently (lines 366-374); on any other error it logs and unwatches (378-388). On a buffer overflow `ReadDirectoryChangesW` succeeds with no bytes, which notify parses as nothing.
