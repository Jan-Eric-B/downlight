# Paste Markdown

Status: ready-for-agent

## Problem Statement

Much of the Markdown I want to read never lives in a file: an answer copied from an AI chat, a snippet from an issue, a pull request or a Teams message. To read it rendered in Downlight today I have to open an editor, paste it, save it somewhere, then open that file, and afterwards delete the file I never wanted. Other Markdown viewers let me paste text straight in. I read such text once and throw it away, so nothing about it needs to be kept, but now and then I'd like to keep one, or edit it and see it rendered again.

## Solution

**Paste**: Ctrl+V, or "Paste" in the right-click menu, shows the plain text on the clipboard rendered in the window, as Open shows a file. Win+V's clipboard history works as well, since it pastes with Ctrl+V. Downlight writes the text to a file of its own in a temporary folder, `%TEMP%\Downlight\Pasted\`, named by the time of the paste (`Pasted 2026-11-20 14.32.05.md`), and the window shows that file like any other document: it joins History, Find, ticks and sorting work, the title bar names the file and reveals it in Explorer, and Edit opens it in the editor, where saving reloads it. The file is temporary: it is deleted once no open window can return to it, and the folder is emptied when Downlight starts. A reader who wants to keep a paste reveals it and saves a copy. A clipboard with no text shows a Notice and changes nothing. The empty window's hint names Paste beside dropping a file.

The decision that a paste is a file Downlight writes, rather than text with no file, is [ADR 0006](../../docs/adr/0006-a-paste-is-a-file-downlight-writes.md).

## User Stories

1. As a reader, I want to press Ctrl+V in a Downlight window to see the Markdown on the clipboard rendered, so that I can read text that isn't in a file.
2. As a reader, I want a "Paste" entry in the right-click menu, so that I can find Paste without knowing its key.
3. As a reader, I want the menu entry to name Ctrl+V, so that I learn the faster way.
4. As a reader, I want the Paste entry beside Open in the menu, so that the ways of giving the window a document are together.
5. As a reader, I want the empty window's hint to say I can paste Markdown with Ctrl+V, so that a new window tells me Paste exists.
6. As a reader, I want Win+V's clipboard history to paste into Downlight, so that I can show something I copied a while ago.
7. As a reader, I want the paste to show in the window I pasted into, so that it appears where I'm looking, as an opened file does.
8. As a reader, I want a paste to replace the document the window shows, so that Paste behaves like Open and a dropped file.
9. As a reader, I want Back to return from a paste to the document I was reading, at its reading position, so that pasting doesn't lose my place.
10. As a reader, I want Forward to return to a paste after Back, so that a paste is a step in History like any document.
11. As a reader, I want a paste into the empty window to show there, so that I can start a window with a paste.
12. As a reader, I want a paste while a document is still on its way to replace it, as Open does, so that I never wait for a document I've moved on from.
13. As a reader, I want Ctrl+V in the Find box to paste into the Find box, so that I can search for copied text as before.
14. As a reader, I want Ctrl+V in any other text field to paste into that field, so that Paste never takes the key from a field I'm typing in.
15. As a reader, I want a held Ctrl+V to paste once, so that a long press doesn't fill History with copies.
16. As a reader, I want only the clipboard's plain text to be pasted, so that what I see is the Markdown I copied, not a web page's formatting.
17. As a reader, I want a clipboard that holds no text (an image, copied files, nothing) to show a Notice saying so and pointing me to dropping a file or Open, so that I know why nothing happened.
18. As a reader, I want a clipboard that holds only spaces or empty lines to count as holding no text, so that I don't get an empty document.
19. As a reader, I want a paste that fails to leave the window showing what it showed, so that a failed paste costs nothing.
20. As a reader, I want a clipboard that another program holds for a moment to be tried again briefly before I'm told it is busy, so that a paste rarely fails for that reason.
21. As a reader, I want the pasted text shown exactly as I copied it, with no guessing that unescapes or reformats it, so that the document is the text I copied.
22. As a reader, I want a large paste to be shown, however large, with the Wait note as a large file gets one, so that a paste is no less able than a file.
23. As a reader, I want the paste's file named by the time I pasted it, so that its name in the title bar tells me what it is and when it came.
24. As a reader, I want two pastes within the same second to get different files, so that one never overwrites the other.
25. As a reader, I want the title bar's file name to reveal the pasted file in Explorer, so that I can find it and keep a copy.
26. As a reader, I want the file name's tooltip to show the pasted file's path and the Counts, as for any document, so that I can see its lines and words.
27. As a reader, I want Edit to open the paste in my editor, so that I can change the text.
28. As a reader, I want saving the paste in my editor to reload it in Downlight, so that I can edit and watch it render.
29. As a reader, I want Find, ticks and sorting to work in a paste, so that a pasted document is as useful as any other.
30. As a reader, I want absolute image paths and web images in a paste to load, as they do in any document, so that a paste with full paths shows its pictures.
31. As a reader, I want relative links and images in a paste to resolve against the temporary folder, as they would in any file there, so that their behavior is never a surprise beyond "there's nothing there".
32. As a reader, I want a pasted file deleted once no open window shows it or can return to it through History, so that my clipboard's text doesn't stay on disk.
33. As a reader, I want a paste I left with Back to stay until I can no longer return to it, so that Forward still works.
34. As a reader, I want a paste dropped from History by going somewhere new after Back to be deleted then, so that it doesn't linger until the window closes.
35. As a reader, I want a paste I stopped on its way to be deleted, so that a paste that never showed leaves nothing behind.
36. As a reader, I want a pasted file that another window also shows, or has in its History, to stay while that window can return to it, so that deleting it never breaks another window.
37. As a reader, I want the pasted folder emptied when Downlight starts, so that pastes left by a crash don't pile up.
38. As a reader, I want emptying the folder at startup not to delay the first paint, so that Paste costs nothing when I open a file.
39. As a reader, I want a pasted file I opened elsewhere through its path to count as shown like any document, so that my window keeps it.
40. As a reader, I want a reload of a deleted paste never to happen, since its window is gone, so that I never see "File deleted or moved" for a paste in a window that still shows it.
41. As a reader on another system, I want Paste to say it isn't available there yet, so that I know it's not broken.
42. As a reader, I want a pasted file's text written as UTF-8 so that any script I copied shows as I copied it.
43. As a reader, I want pasted text that would count as a binary file to be refused with the same message a binary file gets, so that the rule is the same however text arrives.

## Implementation Decisions

- **Glossary.** **Paste** is the term, and **Document** names it as one of the ways a document is given (`GLOSSARY.md`, added during the grilling). Avoid Import, New document and Scratch.
- **A paste is a file Downlight writes** (ADR 0006). Everything after the file is written is the existing path that Open and a drop take: the app gives the window the file with the document table's `give` to that window, so History, the Wait note, Stop, watching, Reload, Edit, the title bar and its reveal need no change.
- **One app command for both ways in.** A new app command, `paste_clipboard`, takes no arguments and answers as `open_document` does: nothing on success, or a Notice. The page calls it for Ctrl+V and for the menu entry alike; the page never reads the clipboard and never sends text, so a large paste is not copied through the page, and Win+V works however the webview treats a paste event with no editable focus. It runs on a background task, keeping the main thread free.
- **Reading the clipboard is the per-OS module's.** The Windows module reads the clipboard's Unicode text (`CF_UNICODETEXT`) through Win32 with the `windows-sys` crate the app already uses, with the features for the clipboard and global memory added; no new dependency and no clipboard plugin. It answers the text, no text (the clipboard holds none, such as an image or copied files), or busy when the clipboard stays held by another program after a few short retries. Other systems' fallback answers that Paste isn't available yet. The `unsafe` code stays in the Windows module.
- **A paste module in the app** owns the pasted folder and hides it: given the clipboard's text, it refuses text that is empty or only whitespace, and text that the core's text check says is binary, then writes the text as UTF-8 without a byte order mark to a new file and returns its path. The file is `Pasted YYYY-MM-DD HH.MM.SS.md` in local time, with a space and `(2)`, `(3)` and so on added before the extension when a file of that name exists. The folder is created when the first paste needs it. Its errors are one type, turned into Notices.
- **The Notices.** No text: "The clipboard holds no text. To open a file, drop it here or use Open." Busy: "The clipboard is in use by another program. Try again." Not available: "Paste isn't available on this system yet." Binary: the binary file's own message, "Downlight opens text files." A failed write names the file and the error, as a failed read does. None of them changes the window.
- **When a pasted file is deleted.** The document table knows every window's shown document, the one on its way and History, so it decides: whenever it lets go of a document (a window closes and is forgotten, going somewhere new after Back drops the forward entries, a document on its way is stopped or replaced, a show fails and the window is put back), each pasted file it let go of that no window still shows, has on its way or holds in its History is deleted, after the table's lock and off the main thread. "Pasted" means its path is in the pasted folder, whichever way it was given, so a pasted file opened again by its path counts as the same file. Deleting a file that is already gone is not an error; any other failure to delete is left for the next startup's sweep and shown to nobody, since the reader can't act on it.
- **The startup sweep.** When the process starts (a second launch hands over to the running process and never sweeps), the pasted folder is emptied on a background task, after the first window's show has started, so first paint doesn't wait for it. It deletes only files last modified before the process started, so a paste another Downlight process writes meanwhile is never swept.
- **The page's command.** A new entry in the page's commands table: label "Paste", shortcut Ctrl+V, running the app command and showing its Notice on failure, not repeating while held. It goes in the right-click menu directly after Open. It is available in every window, with or without a document. Its shortcut is not taken while the focus is in an editable field (the Find box, the settings flyout's fields), so the field gets the key as before; today's table takes every shortcut it matches, so the table learns which shortcuts leave an editable target alone. For the same reason, the menu's Paste is unavailable (greyed out) when the right-click was in an editable field or one has the focus: chosen there, the reader would expect it to paste into the field, as Ctrl+V does.
- **The empty window's hint** reads "or drop a file here, or paste Markdown (Ctrl+V)" under the Open file button.
- **Contract.** The command's answer is the existing nothing-or-Notice shape that Open's has, so no new sample is needed; the page's command types gain `paste_clipboard`, called only from the page's app module. The menu entry is an existing shape.
- **No change to the core.** Render, link resolution and the text check are used as they are.

## Testing Decisions

- A good test drives a public interface as a caller does and checks what the caller or the reader can observe: what a window is given and told, which files exist in the pasted folder, what the page asks the app and the Notice it shows. It never checks a module's internals. Test names state the behavior, such as `a_paste_left_by_going_somewhere_new_after_back_is_deleted`.
- **The app's document table (`cargo test`, the documents tests).** The seam for nearly everything. The tests hand the paste module text in place of the clipboard, in a pasted folder of their own (a temporary folder per test), and check: the window is given a new file in the folder, named by the time, holding the text as UTF-8; two pastes in the same second get two files; a paste joins History and Back returns to the document before it; empty and whitespace-only text, and binary text, send the Notice and leave the window as it was and the folder empty; the file is deleted when its window closes, when going somewhere new after Back drops it, when it is stopped on its way, and when a paste replaces it while it is still on its way; it stays while the window holds it in History, and while another window shows it or holds it in History; a file already gone is no error; the startup sweep empties the folder. Prior art: `app/src/documents/tests/` (`history.rs` for steps and dropping forward entries, `arrival.rs` for Stop, `give.rs` for giving a window a document, the recorder for what pages are told).
- **The page (Vitest with happy-dom, the tests' fake app `fakeTheApp`).** Ctrl+V and the menu entry ask the app to paste; a held Ctrl+V asks once; Ctrl+V with the focus in the Find box or another editable field asks nothing and leaves the key to the field; the menu lists Paste after Open with Ctrl+V; the app's Notice on a failed paste is shown; the empty window's hint names Paste. Prior art: `commands.test.ts` (shortcuts, the menu, repeats), `find.test.ts` (the Find box's keys), `view.test.ts` (the empty state).
- **No unit test for reading the clipboard.** It is Win32 in the per-OS module and talks to the real clipboard, so it is checked by hand, as revealing a file is.
- **Measurement.** Writing the file grows with the paste's size, so pasting the 1 MB test document is measured with the timing switch when it is built, and reported in the ticket's Results.
- **Hand checks in the release build:** Ctrl+V and the menu entry with Markdown copied from a browser, Teams and an editor; Win+V's clipboard history; a clipboard holding an image, copied files, and nothing; a clipboard held by another program; a 20 MB paste and its Wait note; Ctrl+V in the Find box; the title bar's reveal; Edit and a save reloading; closing the window deletes the file; a crash (ending the process) leaves files that the next start sweeps.

## Out of Scope

- Dropping text dragged from another program: its own ticket. Tauri's native drop gives only file paths, and taking dragged text likely means rebuilding file drops in the page.
- Reading from stdin or `--text` on the command line, which stay out as in the v1 spec.
- Pasting anything but plain text: HTML, images, or files copied in Explorer (which a drop or Open shows).
- Unescaping text that looks like a JSON string (mdhero's "LLM mode"), or any other guess that changes the pasted text.
- Saving a paste from Downlight (a Save or Save as command): the reader reveals the file and copies it.
- Naming the file after its first heading.
- A size limit for pastes.
- Recent documents listing pastes: when it is built it skips them (noted on its ticket).
- Other systems: they keep the plain fallback until their ports.

## Further Notes

- From the v1 spec's Out of Scope (v1.x), first a stub ticket (now replaced by this spec's tickets), grilled with the repo owner on 2026-11-20.
- The converter (`.scratch/converter/`) asks the same question of a document without a file; ADR 0006's answer, a file Downlight writes, may serve it too.
