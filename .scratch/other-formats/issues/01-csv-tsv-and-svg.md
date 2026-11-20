# 01: CSV and TSV as a table, SVG as an image

**What to build:** Downlight shows a CSV or TSV file as a table and an SVG file as an image, instead of as Markdown text. Downlight shows up in "Open with" for them but never asks to be their default app.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later ("CSV/TSV as a table and SVG as an image first, then JSON and XML"). Research: [research.md](../research.md).

**Blocked by:** None (can start immediately). Ticket 03 (text and code) sets up rendering chosen by format too; whichever comes first builds it.

**Status:** needs-triage

- [ ] Grill it before planning. Open questions, with what the research found:
  - The glossary's **Document** is "rendered as Markdown". These aren't, so the term widens, or a new one comes (**Format**?).
  - Format by extension or by content? The v1 spec's story 83 already wants rendering chosen by format behind one entry point in the core (`core/src/format.rs`).
  - SVG: shown as an `<img>` of its own file through the image scheme, which already serves `.svg` as `image/svg+xml` (`core/src/image.rs`). The SVG specs answer the safety question: an SVG an `img` references runs in secure animated mode, with scripts disabled, no external file loaded and no interactivity, while CSS and SMIL animation still play. No sanitizer of its own is needed. The cost: an SVG that loads outside fonts or images shows without them.
  - SVG's view: fit to window, 100 % and zoom; a background switch (checkerboard, light, dark), since an SVG drawn in black on transparent vanishes in the dark theme; its size and `viewBox`; its source as highlighted XML (ticket 03's text view).
  - CSV parsing: the delimiter detected (`,`, `;` as Excel writes it in many European locales, tab, `|`), quoted fields with line breaks, encodings (decoding already handles a byte order mark and Windows-1252).
  - The header row: GitHub always takes the first row; Tad has a "no header row" option. A guess with a switch, or always the first row?
  - Rows with different numbers of fields: GitHub refuses the file with an error. Show them anyway, padded, with a **Notice**?
  - Line numbers: a quoted field can span lines, so each record carries the line it starts on, for the reading position, Edit and Reload's anchoring.
  - Very large files (a 100 MB CSV). GitHub stops at 512 KB, which Downlight can't, since every document is shown. ADR 0003 lays out only top-level blocks near the view, and one `<table>` is one block, so the rows have to come in many blocks (a table per few hundred rows with shared column widths) to be laid out lazily. The fallback, a grid that draws only the rows in view (Modern CSV, CSV Buddy), breaks "the core renders HTML" and comes only after a measurement on the large documents.
  - **Sort** (table-sorting/01), filter (table-sorting/02) and the sticky header (table-sorting/03) on the table, rather than features of this ticket's own.
  - Small additions: numbers aligned right, a long cell cut short with the whole value in a tooltip, the rows and columns in the **Counts**, copying a selection as TSV so it pastes into Excel as cells, links to a row as GitHub has. Which belong in this ticket?
  - Which features apply: Find, Edit, **Reload** (a CSV or SVG written by a script becomes a live preview), the **Overview ruler**.
  - The installer: "Open with" entries for `.csv`, `.tsv` and `.svg`, claiming none of them.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). JSON and XML follow in ticket 02.

**2026-11-20.** Open questions extended from a web search of other viewers (GitHub, Tad, Modern CSV, CSV Buddy, PowerToys) and the SVG specs; the findings and sources are in [research.md](../research.md).
