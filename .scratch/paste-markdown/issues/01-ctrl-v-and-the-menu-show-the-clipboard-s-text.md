# 01: Ctrl+V and the menu show the clipboard's text

**What to build:** The reader presses Ctrl+V, or picks "Paste" in the right-click menu, and the window shows the clipboard's plain text rendered, as Open shows a file (**Paste**). Downlight writes the text to a file of its own in its pasted folder, named by the time of the paste, and gives the window that file: it joins **History**, Back returns to the document before it, and the title bar names it and reveals it. Win+V's clipboard history pastes the same way. A clipboard with no text, a busy clipboard, or text that counts as a **Binary file** shows a **Notice** and leaves the window as it was. The empty window's hint names Paste. Pasted files are not deleted yet; ticket 02 does that.

Spec: [Paste Markdown](../spec.md), [ADR 0006](../../../docs/adr/0006-a-paste-is-a-file-downlight-writes.md)

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] A new app command, `paste_clipboard`, takes no arguments and answers as `open_document` does (nothing, or a Notice). It runs on a background task: it reads the clipboard, has the paste module write the file, and gives the window the file with the document table's `give` to that window, as Open and a drop do.
- [ ] The Windows module reads the clipboard's Unicode text through Win32 (`windows-sys`, with the clipboard and global memory features added; no new dependency), answering the text, no text, or busy after a few short retries while another program holds the clipboard. Other systems' fallback answers that Paste isn't available yet. The `unsafe` code stays in the Windows module.
- [ ] The paste module owns the pasted folder (`%TEMP%\Downlight\Pasted\`, created when the first paste needs it): it refuses empty and whitespace-only text and text the core's text check calls binary, and writes the rest as UTF-8 without a byte order mark to `Pasted YYYY-MM-DD HH.MM.SS.md` in local time, adding a space and `(2)`, `(3)` and so on before the extension when that name is taken. Its errors are one type, turned into Notices.
- [ ] The Notices: "The clipboard holds no text. To open a file, drop it here or use Open."; "The clipboard is in use by another program. Try again."; "Paste isn't available on this system yet."; binary text gets "Downlight opens text files."; a failed write names the file and the error. None changes the window.
- [ ] The page's commands table gains "Paste", shortcut Ctrl+V, not repeating while held, available in every window, running the app command and showing its Notice. It sits in the right-click menu directly after Open.
- [ ] Ctrl+V with the focus in an editable field (the Find box, the settings flyout's fields) is left to the field: the commands table learns which shortcuts leave an editable target alone, and Paste's is one.
- [ ] The empty window's hint reads "or drop a file here, or paste Markdown (Ctrl+V)".
- [ ] The page's command types gain `paste_clipboard`, called only from the page's app module. Its answer is Open's existing shape, so no new contract sample.
- [ ] Documents tests, with the paste module given text and a pasted folder of the test's own: the window is given a new file in the folder named by the time and holding the text as UTF-8; two pastes in the same second get two files; a paste joins History and Back returns to the document before it, at its reading position; a paste into the empty window shows there; a paste replaces a document still on its way; empty, whitespace-only and binary text send their Notice and leave the window and the folder as they were.
- [ ] Page tests (`fakeTheApp`): Ctrl+V and the menu entry ask the app to paste; a held Ctrl+V asks once; Ctrl+V in the Find box asks nothing and the box gets the key; the menu lists Paste after Open with Ctrl+V; a failed paste shows the app's Notice; the empty window's hint names Paste.
- [ ] Pasting the 1 MB test document is measured with the timing switch, and the medians are written under Results below.
- [ ] Hand checks in the release build: Ctrl+V and the menu with Markdown copied from a browser, Teams and an editor; Win+V's clipboard history; a clipboard holding an image, copied files, and nothing; a clipboard held by another program; a 20 MB paste and its **Wait note**; Ctrl+V in the Find box; the title bar's reveal of the pasted file; Edit, and a save in the editor reloading.
- [ ] `npm run check` passes.

## Results

(to be filled in when built)
