# Copy as Markdown, and a clean rich copy

Status: ready-for-agent

## Problem Statement

When I read a document in Downlight and want to pass part of it on, I can copy only the text I see. Pasted into another Markdown file, a chat, an issue or a pull request, that text has lost its Markdown: headings, emphasis, links, lists, tables and code fences are gone, so I open the file in an editor, find the same lines and copy them there. The other way round, when I paste what I copied into Word or Outlook, I want the formatting I see, but what the webview puts on the clipboard carries Downlight's own look with it (in the dark theme, light text meant for a dark background), local images that only Downlight can load, links that lead nowhere outside Downlight, and task items without their boxes.

## Solution

**Copy as Markdown**: Ctrl+Shift+C, or "Copy as Markdown" in the right-click menu directly after Copy, puts the document's Markdown on the clipboard instead of the text the reader sees. With a selection, it copies the file's whole lines of the blocks the selection touches. Inside a code block or a table, it copies only the part selected:

- a selection within one code block copies exactly the selected text, as Ctrl+C does;
- a selection that runs into or out of a code block copies the code lines it touches, inside the block's own fences;
- a selection within one table cell copies that cell's Markdown;
- a selection across cells, or into a table from outside, copies the table's header and delimiter rows and the rows it touches.

The whitespace every copied line starts with is dropped, so a nested list pastes as a list, keeping its inner indentation. With nothing selected, it copies the whole file. It copies the file as the shown render was made from it, as written: a **Sort** or a **Tick** never changes it, and a file changed or deleted since keeps copying what is shown. It is unavailable while the window shows no document.

**A clean rich copy**: Ctrl+C, and the menu's Copy, keep putting the same plain text on the clipboard, and put clean HTML beside it that Word and Outlook give their own styles:

- headings, emphasis, lists, tables, quotes and code, with none of Downlight's colors, theme, copy buttons or heading anchors;
- code as plain monospace, without highlighting;
- alerts as quotes with their title;
- local images by their file path, web images by their URL, images that don't load as their alt text;
- web and mail links as links, every other link as its text;
- ticks as ☑ and ☐;
- a sorted table in the order shown.

## User Stories

1. As a reader, I want to copy the Markdown of the part of a document I selected, so that I can paste it into another Markdown file, a chat or an issue with its formatting intact.
2. As a reader, I want Copy as Markdown on Ctrl+Shift+C, so that it sits beside Ctrl+C and I can reach it without the mouse.
3. As a reader, I want "Copy as Markdown" in the right-click menu directly after Copy, so that I find it where I already copy.
4. As a reader, I want the menu entry to name Ctrl+Shift+C, so that I learn the faster way.
5. As a reader, I want Ctrl+C to keep copying the text I see, so that what I'm used to doesn't change.
6. As a reader, I want a selection that starts or ends in the middle of a paragraph to copy the paragraph's whole lines, so that the copy is valid Markdown rather than a fragment cut through a link or emphasis.
7. As a reader, I want a selection across several blocks to copy every line from the first block it touches to the last, so that the blank lines and structure between them come along.
8. As a reader, I want a selection inside one list item to copy that item's lines with its marker (`- `, `1. `, `- [ ] `), so that it pastes as a list item.
9. As a reader, I want a selection inside a nested list to lose only the indentation all its lines share, so that it pastes as a list, not as a code block, and its inner items stay indented under their parents.
10. As a reader, I want a selection inside a quote to keep its `> ` markers, so that it pastes as a quote.
11. As a reader, I want a selection within one code block to copy exactly the text I selected, so that I get the code I chose and not the fences around it.
12. As a reader, I want a selection that runs from a paragraph into a code block to copy the paragraph and the code lines I touched, inside the block's own fences with its language, so that the paste stays valid Markdown holding only what I chose.
13. As a reader, I want a selection that runs out of a code block into the text after it to copy the code lines I touched from the opening fence on, so that the code still sits in its fence.
14. As a reader, I want the document's front matter to copy as it is written, between its `---` lines, when my selection runs into or out of it, so that it pastes as front matter.
15. As a reader, I want the copy button on a code block to keep copying the whole block's code, so that both ways of copying code stay.
16. As a reader, I want a selection within one table cell to copy that cell's Markdown, so that I get its links and emphasis without the pipes around it.
17. As a reader, I want a selection across several cells or rows to copy the table's header and delimiter rows with the rows I touched, so that the paste is a table of only those rows.
18. As a reader, I want a selection that runs from a paragraph into a table to copy the paragraph and the table's header, delimiter and touched rows, so that the table part is still a table.
19. As a reader, I want Copy as Markdown of a sorted table to give its rows in the file's order, so that it copies the file as written, while Ctrl+C gives the order I see.
20. As a reader, I want Copy as Markdown to copy task items as the file writes them, whatever I ticked, so that ticks, which are never written to the file, never reach the copy either.
21. As a reader, I want Copy as Markdown with nothing selected to copy the whole file, so that I can pass on a whole document in one step.
22. As a reader, I want Copy as Markdown to copy the file as the shown document was rendered from it, so that the copy matches what I see even while a reload is on its way.
23. As a reader, I want Copy as Markdown to keep working after the document's file was deleted or moved, so that the file-gone bar doesn't stop me copying what I still see.
24. As a reader, I want Copy as Markdown after a reload to copy the new text, so that it follows the document I see.
25. As a reader, I want a file in UTF-16 or Windows-1252 to copy as the characters I see, so that the copy never has broken characters.
26. As a reader, I want Copy as Markdown of a large selection, even the whole of a large document, to be copied without passing it through the page, so that it is quick.
27. As a reader, I want Copy as Markdown to be unavailable, greyed out in the menu, while the window shows no document (the empty window, or a first document still on its way), so that it never seems to do something it can't.
28. As a reader, I want Copy as Markdown while another document is on its way to copy the one in view, so that it always copies what I see.
29. As a reader, I want Copy as Markdown to show nothing when it works, as Copy shows nothing, so that copying stays quiet.
30. As a reader, I want a Notice when Copy as Markdown fails, such as when another program holds the clipboard, so that I know nothing was copied and can try again.
31. As a reader, I want a held Ctrl+Shift+C to copy once, so that a long press does nothing odd.
32. As a reader, I want Ctrl+Shift+C in the Find box or a settings field to do nothing special, so that a field I'm typing in keeps its keys.
33. As a reader on another system, I want Copy as Markdown to say it isn't available there yet, so that I know it's not broken.
34. As a reader, I want Ctrl+C pasted into Word or Outlook to keep the document's headings, emphasis, lists, tables, quotes and code, so that I can pass on a document with its formatting.
35. As a reader, I want that paste to take the target's own styles, never Downlight's colors or theme, so that a copy made in the dark theme is readable on a white page.
36. As a reader, I want code in that paste as plain monospace text, so that the dark theme's highlighting colors don't land on a white page.
37. As a reader, I want alerts in that paste as quotes with their title ("Note", "Warning"), so that they keep their meaning without Downlight's styles.
38. As a reader, I want footnotes in that paste as a numbered list, so that their text comes along.
39. As a reader, I want that paste to leave out Downlight's own parts (copy buttons, heading anchors, Find's highlights), so that only the document is pasted.
40. As a reader, I want local images in that paste by their file path, so that Word and Outlook on this computer can show them.
41. As a reader, I want web images in that paste by their URL, so that they show where the target loads them.
42. As a reader, I want an image that doesn't load in Downlight, such as one on a network share, to paste as its alt text, so that the paste never reaches somewhere Downlight wouldn't.
43. As a reader, I want web and mail links in that paste to stay links, so that they work in the target.
44. As a reader, I want every other link (a heading in the document, a footnote, another document or file) to paste as its text, so that the target holds no links that lead nowhere.
45. As a reader, I want task items in that paste to show ☑ or ☐ as I ticked them, so that their boxes survive a target that drops checkboxes.
46. As a reader, I want a sorted table in that paste in the order I see, so that Ctrl+C copies what is shown.
47. As a reader, I want the plain text that Ctrl+C copies to be exactly what it copied before, so that pasting into Notepad or a terminal doesn't change.
48. As a reader, I want Ctrl+A then Ctrl+C to copy the whole document that way, so that I need no separate command to copy a document as rich text.
49. As a reader, I want copying in the Find box to copy its text as before, so that the rich copy is only the document's.
50. As a reader, I want Copy and Copy as Markdown to stay quick on large documents, so that copying never stalls the window.

## Implementation Decisions

- **Glossary.** **Copy as Markdown** is the term (added during the grilling). Avoid Copy source and Copy raw. The rich copy is what Ctrl+C does and gets no term of its own.
- **Pieces of the file.** The page tells the app what to copy as the pieces of the file it covers, never as text. A piece is either a range of whole lines (first and last line, 1-based) or a span within one line (the line, and its first and last column as the rendered document's line numbers count them, which is how comrak's `data-sourcepos` counts a cell). No pieces means the whole file. This is the one new shape between the page and the app.
- **The page maps the selection to pieces** (a new page module for Copy as Markdown). It works from the line numbers every rendered block carries, and only blocks carry them:
  - It finds the innermost blocks containing the selection's start and end, and copies the lines from the first's start to the last's end.
  - A code block or a table the selection covers only partly is replaced by its own pieces. For a code block, those are its opening fence line, the code lines touched and its closing fence line. The code line comes from counting the code's lines before the selection's point. A fenced block is told from an indented one by comparing the block's lines with its code's lines, and front matter counts as fenced. For a table, the pieces are its header row's line, the delimiter row's line after it, and each touched row's line.
  - The pieces are put in the file's order, so a sorted table copies in the file's order.
  - A selection within one code block is not sent: the page writes the selected text to the clipboard itself, as the copy buttons do.
  - A selection within one cell is that cell's span.
  - No selection, or a selection outside the document view, means the whole file.
- **The core's new copy module.** One public function takes the bytes a render took and the pieces, and returns the Markdown:
  - It decodes the bytes as render does.
  - It drops the whitespace every non-blank copied line starts with, and keeps every other prefix (`> `, list markers).
  - It joins the lines with `\n`.
  - A cell's span is trimmed of the spaces around it.
  - A piece outside the file, or a span whose columns fall outside its line or inside a character, is its error type, one for the feature.
  - The core reads no file: the app passes it the bytes.
- **The app keeps the shown render's bytes.** The document table keeps, for each window, the bytes of the render its page took last, with that render's showing.
  - Reading and rendering returns the bytes with the rendered document, shared rather than copied.
  - They are replaced when the page takes a reload's or another document's render. They are dropped when the window shows no document or closes.
  - This costs about the document's size in memory per window, accepted so that a copy matches what is shown while the file changes or is gone.
- **One app command.** `copy_markdown` takes the showing the view shows and the pieces, and answers as `open_document` does: nothing on success, or a Notice.
  - A showing other than the one whose bytes it keeps copies nothing and shows nothing: the reader has moved on.
  - It runs on a background task, so the main thread stays free.
  - The page never receives the Markdown, so a large copy never passes through it.
- **Writing the clipboard is the per-OS module's.** The Windows module writes Unicode text (`CF_UNICODETEXT`) through Win32 with the `windows-sys` crate the app already uses, with the clipboard and global memory features. Paste's ticket adds the same features to read the clipboard, so whichever is built first adds them.
  - It writes line breaks as `\r\n`, as Windows text expects.
  - It retries a few times while another program holds the clipboard, and then answers busy.
  - Other systems' fallback answers that Copy as Markdown isn't available yet.
  - The `unsafe` code stays in the Windows module.
- **The Notices.** Busy: "The clipboard is in use by another program. Try again." Not available: "Copy as Markdown isn't available on this system yet." The core's error, which only a stale or broken request can cause: "Can't copy the Markdown: the selection is no longer in the document."
- **The page's command.** A new entry in the page's commands table:
  - label "Copy as Markdown", shortcut Ctrl+Shift+C;
  - available while the window shows a document;
  - not repeating while held;
  - leaving editable fields alone, as Paste's ticket teaches the table, whichever is built first;
  - shows the app's Notice on failure;
  - placed in the right-click menu directly after the system's Copy.
- **The rich copy** (a new page module). It listens for the copy event, which Ctrl+C and the menu's Copy both raise. When the selection lies in the document view, it cancels the webview's own copy and sets two flavors itself:
  - **Plain text:** the selection's text as the webview copied it before.
  - **HTML:** rebuilt from the selected part of the view, with an allowlist:
    - Kept: the structural elements (headings, paragraphs, emphasis, lists, quotes, tables, `pre` and `code`, images, links, footnotes), with no classes, styles or ids.
    - Removed: copy buttons, heading anchors and Find's highlights (their text stays).
    - Alerts become `blockquote`s whose first paragraph is their title.
    - A task item's checkbox becomes ☑ or ☐ followed by a space, by its tick.
    - Images: a local image's image-scheme URL becomes its `file:///` URL, a web image keeps its URL, and an image without a source becomes its alt text.
    - Links: an `http(s)` or `mailto` link stays a link, and any other becomes its text.
    - A sorted table is taken in the order the view shows it.

  A copy in the Find box or any other field is left to the webview.
- **Contract.** `copy_markdown`'s arguments are typed in the page's app module, the only one that calls `invoke`. The app's tests write a sample of the pieces it takes (a line range, a span, and the whole file), as they do for the menu entries. The page's contract test checks the keys it sends against them. The answer is the existing nothing-or-Notice shape, and the menu entry is an existing shape.

## Testing Decisions

- A good test drives a public interface as a caller does and checks what the caller or the reader can observe: the Markdown the core returns, what the app writes to the clipboard and tells the page, the pieces the page sends, and what the page puts on the clipboard. It never checks a module's internals. Test names state the behavior, such as `a_selection_into_a_code_block_copies_its_touched_lines_inside_its_fences`.
- **The core's public interface (`cargo test -p downlight-core`).** Tests use real Markdown, with each piece's line numbers taken from the core's own render, so the two can't drift apart. Covered:
  - whole-line ranges, and the whole file;
  - a cell's span, trimmed;
  - a table's header, delimiter and touched rows;
  - code lines inside their fences, front matter's `---` lines included;
  - shared indentation dropped, relative indentation and `> ` kept, and blank lines ignored when finding what all lines share;
  - UTF-16 and Windows-1252 files;
  - a piece outside the file, and a span inside a character, give the error.

  Prior art: the core's render tests and `core/tests/`.
- **The app's document table (`cargo test`, the documents tests).** The per-OS clipboard writer is replaced by a recorder. Covered:
  - a copy writes the shown render's Markdown;
  - it still does after the file changed on disk before a reload was taken, and after the file was deleted;
  - after the page took a reload's render, it writes the new text;
  - a showing the window has left writes nothing and sends nothing;
  - the window showing no document writes nothing;
  - a busy clipboard sends the Notice;
  - closing the window drops the kept bytes.

  Prior art: `app/src/documents/tests/` (`handover.rs` for the render the page took, `watch.rs` for a changed and a deleted file, `give.rs`), the recorder for what pages are told, and the paste spec's replaced clipboard reader.
- **The page (Vitest with happy-dom, the tests' fake app `fakeTheApp`, the rendered fixtures).** Selections are made in the committed HTML of `core/tests/fixtures/`, so the line numbers are the core's. Covered:
  - the pieces sent for a selection across paragraphs, inside a list item, running into and out of a code block, within one cell, across rows (and across rows of a sorted table, in the file's order), and nothing selected;
  - a selection within one code block writes its text without asking the app;
  - the menu lists Copy as Markdown after Copy with Ctrl+Shift+C, greyed out with no document;
  - a held Ctrl+Shift+C asks once, and in the Find box asks nothing;
  - the app's Notice is shown;
  - the copy event in the document view sets the clean HTML for each rule above (colors and classes gone, copy buttons and anchors gone, alerts, ticks as ☑ and ☐, local, web and unloaded images, kept and dropped links, a sorted table's order), and sets the plain text;
  - a copy in the Find box is left alone.

  Prior art: `copy.test.ts` (the copy buttons), `commands.test.ts` (shortcuts, the menu, repeats), `ticks.test.ts`, `contract.test.ts`.
- **No unit test for writing the clipboard.** It is Win32 in the per-OS module and talks to the real clipboard, so it is checked by hand, as Paste's reading is.
- **Measurement.** Both copies grow with the selection's size. Copy as Markdown and Ctrl+C of the whole 1 MB test document (Ctrl+A first) are measured with the timing switch when they are built, and reported in the tickets' Results.
- **Hand checks in the release build:**
  - Copy as Markdown pasted into Notepad, VS Code, Teams and a GitHub comment, from a paragraph, a nested list, a code block, a table cell, rows of a sorted table, and the whole document;
  - a file changed and deleted since it was shown;
  - a clipboard held by another program;
  - before the rich copy is built: what today's Ctrl+C pastes into Word in the dark theme (expected: the theme's colors come along);
  - after it: Ctrl+C and the menu's Copy pasted into Word and Outlook in both themes, with local and web images, links, ticks and a sorted table;
  - the plain text pasted into Notepad, compared with the build before it on the same selections;
  - copying in the Find box.

## Out of Scope

- Copying exactly the selected characters of a paragraph, list item or heading: Copy as Markdown copies their whole lines, since only blocks carry line numbers.
- Copying the section under a heading: it may come with the Outline (`.scratch/outline/`).
- A separate "Copy as rich text" or "Copy document" command: Ctrl+C is the rich copy, and Ctrl+A selects the whole document.
- Keeping code highlighting colors, or any of Downlight's look, in the rich copy.
- Embedding images in the rich copy's HTML (as `data:` URLs): local images go by their file path.
- Writing Markdown from what is shown (ticks as ticked, a table as sorted): that is the converter's question (`.scratch/converter/`).
- Exporting a document as an HTML file: its own ticket (`.scratch/export-html/`).
- A title bar button for either copy.
- Other systems: they keep the plain fallback until their ports. The rich copy is the page's and works wherever the page runs.

## Further Notes

- From the v1 spec's Out of Scope (v1.x), first a stub ticket (now replaced by this spec's tickets), grilled with the repo owner on 2026-11-20.
- Settles the open question in the table-sorting ticket's note: Ctrl+C copies a sorted table in the shown order, and Copy as Markdown copies it in the file's order.
- The rich copy's one risk is the plain text: cancelling the webview's copy means the page sets the plain text itself, and Chromium's plain text for a copy can differ in small ways from the selection's text (a non-breaking space, an image's alt text). The hand check compares the two. If they differ, the plain text is made to match the build before it, since the promise is that Ctrl+C's plain text doesn't change.
- When the line numbers in the margin are built (`.scratch/line-numbers/`), the rich copy leaves them out, as it leaves out the copy buttons.
