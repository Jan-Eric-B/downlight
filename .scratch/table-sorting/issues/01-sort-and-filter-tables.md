# 01: Sort, and maybe filter, a document's tables

**What to build:** The reader can sort a table in the shown document by one of its columns, and perhaps filter its rows, to read a large table (a changelog, a comparison, a list of settings) in the order they need. Like a tick, the order belongs to the window and is never written to the file.

Spec: none yet. Grill it first (`/grill-with-docs`), then `/to-spec` writes `.scratch/table-sorting/spec.md`.

**Blocked by:** none

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - **Sort only, or filter too.** Find already looks for text in the shown document and only there (GLOSSARY); a row filter is a second way to look for text, with its own box and its own rules for what matches. Sorting alone may be enough.
  - **How it is started:** a click on a column's header (GitHub doesn't do this, so it changes how a document's tables look and behave), a button that shows on hover, or a command on the right-click menu.
  - **How values compare:** text, numbers, dates and versions (`1.10` after `1.9`), empty cells, and cells holding links, code or formatting.
  - **Which tables:** a table with merged or missing cells, or a header row only, and tables written as raw HTML.
  - **What else follows the order:** the line numbers in the margin (each row's line in the file), Find's matches and the current match, the reading position on a reload, Back and Forward, and the overview ruler.
  - **What a reload does:** keeps the order (as a reload keeps ticks) or shows the file's order again.
- [ ] Measured with the timing switch on a table of several thousand rows in the 1 MB test document, when it is built.

## Comments

**2026-11-17, from the repo owner.** Asked for as a feature idea. It fits "reading only", since the file never changes. It is outside the v1 spec, so it gets its own feature folder rather than a v1 ticket; the v1 spec's Out of Scope may want a line for it after the grilling.
