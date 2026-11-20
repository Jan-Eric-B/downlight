# 01: Ctrl+Shift+C and the menu copy the Markdown of whole lines

**What to build:** The reader presses Ctrl+Shift+C, or picks "Copy as Markdown" in the right-click menu, and the clipboard gets the document's Markdown instead of the text they see (**Copy as Markdown**). A selection copies the file's whole lines, from the first block it touches to the last, dropping only the whitespace every copied line starts with, so a nested list pastes as a list. With nothing selected, it copies the whole file. It copies the file as the shown render was made from it: a **Sort** or a **Tick** never changes it, and a file changed or deleted since keeps copying what is shown. It is unavailable while the window shows no document. A code block or a table the selection touches copies its whole lines for now; tickets 02 and 03 copy only the selected part.

Spec: [Copy as Markdown, and a clean rich copy](../spec.md)

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The core gains a copy module with one public function: given the bytes a render took and the pieces of the file (ranges of whole lines, 1-based; no pieces for the whole file), it returns the Markdown. It decodes the bytes as render does, drops the whitespace every non-blank copied line starts with (keeping `> ` and list markers), and joins the lines with `\n`. A piece outside the file is its error type, one for the feature.
- [ ] Reading and rendering returns the file's bytes with the rendered document, shared rather than copied. The document table keeps, for each window, the bytes of the render its page took last with that render's showing. It replaces them when the page takes a reload's or another document's render, and drops them when the window shows no document or closes.
- [ ] A new app command, `copy_markdown`, takes the showing the view shows and the pieces, and answers as `open_document` does (nothing, or a Notice). It runs on a background task. A showing other than the kept one copies nothing and sends nothing. The page never receives the Markdown.
- [ ] The Windows module writes Unicode text to the clipboard through Win32 (`windows-sys` with the clipboard and global memory features; paste-markdown/01 adds the same for reading, so whichever is built first adds them). It writes line breaks as `\r\n` and answers busy after a few short retries while another program holds the clipboard. Other systems' fallback answers that Copy as Markdown isn't available yet. The `unsafe` code stays in the Windows module.
- [ ] The Notices: "The clipboard is in use by another program. Try again."; "Copy as Markdown isn't available on this system yet."; the core's error gives "Can't copy the Markdown: the selection is no longer in the document."
- [ ] A new page module maps the selection to pieces from the line numbers the rendered blocks carry: the lines from the start of the innermost block containing the selection's start to the end of the one containing its end. No selection, or a selection outside the document view, sends no pieces (the whole file).
- [ ] The page's commands table gains "Copy as Markdown", shortcut Ctrl+Shift+C, available while the window shows a document, not repeating while held, showing the app's Notice. It sits in the right-click menu directly after the system's Copy. Ctrl+Shift+C with the focus in an editable field (the Find box, the settings flyout's fields) is left to the field: the commands table learns which shortcuts leave an editable target alone, as paste-markdown/01 also needs, whichever is built first.
- [ ] Contract: `copy_markdown` is typed in the page's app module. The app's tests write a sample of the pieces it takes (a line range, and the whole file), and the page's contract test checks the keys it sends against them. The answer is Open's existing shape.
- [ ] Core tests (real Markdown, line numbers taken from the core's own render): whole-line ranges and the whole file; shared indentation dropped, relative indentation and `> ` kept, blank lines ignored when finding the shared indentation; UTF-16 and Windows-1252 files; a piece outside the file gives the error.
- [ ] Documents tests, with the clipboard writer replaced by a recorder: a copy writes the shown render's Markdown; it still does after the file changed on disk before a reload was taken, and after the file was deleted; after the page took a reload's render it writes the new text; a showing the window has left writes and sends nothing; a window with no document writes nothing; a busy clipboard sends the Notice; closing the window drops the kept bytes.
- [ ] Page tests (`fakeTheApp`, selections made in the rendered fixtures): the pieces sent for a selection across paragraphs, inside a list item, inside a nested list, and nothing selected; the menu lists Copy as Markdown after Copy with Ctrl+Shift+C, greyed out with no document; a held Ctrl+Shift+C asks once; in the Find box it asks nothing; the app's Notice is shown.
- [ ] Copy as Markdown of the whole 1 MB test document (Ctrl+A first) is measured with the timing switch, and the medians are written under Results below.
- [ ] Hand checks in the release build: Copy as Markdown pasted into Notepad, VS Code, Teams and a GitHub comment, from a paragraph, a nested list and the whole document; a file changed and deleted since it was shown; a clipboard held by another program.
- [ ] `npm run check` passes.

## Results

(to be filled in when built)
