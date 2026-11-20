# 03: Copy as Markdown of part of a table

**What to build:** **Copy as Markdown** of a selection within one table cell copies that cell's Markdown, without the pipes around it. A selection across cells or rows, or into a table from outside, copies the table's header and delimiter rows and the rows it touches, and no others, so the paste is a table of only those rows. The rows come in the file's order even when the reader has sorted the table (**Sort**), since Copy as Markdown copies the file; Ctrl+C gives the order shown.

Spec: [Copy as Markdown, and a clean rich copy](../spec.md)

**Blocked by:** 01 (Ctrl+Shift+C and the menu copy the Markdown of whole lines)

**Status:** ready-for-agent

- [ ] Pieces gain a span within one line: the line, and its first and last column as the rendered document's line numbers count them (comrak's `data-sourcepos` on a cell). The core copies a span trimmed of the spaces around it. A span whose columns fall outside its line or inside a character is the copy module's error.
- [ ] A selection within one cell sends that cell's span.
- [ ] A table the selection covers only partly becomes its own pieces: its header row's line, the delimiter row's line after it, and each touched row's line, in the file's order whatever order the view shows the rows in. A selection from a paragraph into a table, or out of one, copies the paragraph's lines and the table's pieces.
- [ ] Contract: the app's sample of the pieces it takes gains a span, and the page's contract test checks its keys.
- [ ] Core tests: a cell's span, trimmed, with emphasis and a link in it; a span inside a character gives the error; a table's header, delimiter and touched rows from pieces the page would send.
- [ ] Page tests (rendered fixtures): the span sent for a selection within one cell; the pieces sent for a selection across cells of one row, across rows, and from a paragraph into a table; with the view's rows reordered, as a sort reorders them, the pieces still come in the file's order.
- [ ] Hand checks in the release build: Copy as Markdown pasted into VS Code and a GitHub comment from within a cell, across rows, and from a paragraph into a table; once tables/01 is built, across rows of a sorted table.
- [ ] `npm run check` passes.

## Results

(to be filled in when built)
