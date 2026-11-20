# 02: A pasted file is deleted once no window can return to it

**What to build:** A pasted document's file is temporary (**Paste**, ADR 0006). It is deleted once no open window shows it, has it on its way, or holds it in its **History**: when its window closes, when going somewhere new after Back drops it from History, when the reader stops it on its way (**Stop**), or when another document replaces it before it arrived. While any window can still return to it, Back and Forward included, it stays. When Downlight starts, the pasted folder is emptied, to clear up after a crash, without delaying **First paint**.

Spec: [Paste Markdown](../spec.md), [ADR 0006](../../../docs/adr/0006-a-paste-is-a-file-downlight-writes.md)

**Blocked by:** 01 (Ctrl+V and the menu show the clipboard's text)

**Status:** ready-for-agent

- [ ] The document table decides: whenever it lets go of a document (a window forgotten on closing, forward entries dropped by going somewhere new after Back, a document on its way stopped or replaced, a failed show putting the window back), each pasted file it let go of that no window still shows, has on its way or holds in its History is deleted. Deleting happens after the table's lock and off the main thread.
- [ ] "Pasted" means its path is in the pasted folder, whichever way it was given, so a pasted file opened again by its path (Ctrl+O, a drop) counts as the same file.
- [ ] A file already gone is no error. Any other failure to delete is left for the next startup's sweep and shown to nobody, since the reader can't act on it; a comment says so.
- [ ] When the process starts (not a second launch that hands over to the running process), the pasted folder is emptied on a background task, after the first window's show has started.
- [ ] Documents tests: the file is deleted when its window closes; when going somewhere new after Back drops it; when it is stopped on its way; when a paste or another document replaces it on its way; it stays while its window holds it in History (Back from it, then Forward returns to it); it stays while another window shows it or holds it in History, and goes when the last of them lets go; a file already gone is no error; a document outside the pasted folder is never deleted; the startup sweep empties the folder.
- [ ] Hand checks in the release build: closing a paste's window deletes its file; ending the process from Task Manager leaves its pastes, and the next start sweeps them; first paint from the file manager is no slower with pastes left in the folder.
- [ ] `npm run check` passes.
