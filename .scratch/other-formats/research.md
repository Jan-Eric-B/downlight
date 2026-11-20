# Other formats: how other viewers show them

Written 2026-11-20 from a web search and a read of this repo's core (`core/src/format.rs`, `core/src/image.rs`, ADR 0003, ADR 0004). Nothing below was installed or run; each claim about another app comes from its own docs or a review, and vendor speed claims are only the vendor's. It feeds the open questions of tickets [01](issues/01-csv-tsv-and-svg.md), [02](issues/02-json-and-xml.md), [03](issues/03-text-and-code.md) and [04](issues/04-jupyter-notebooks.md).

## 1. What's out there

| Format | Viewers | How they show it |
| --- | --- | --- |
| CSV, TSV | GitHub, NirSoft CSVFileView, Modern CSV, CSV Buddy, Lazy CSV Viewer, Tad, Rainbow CSV (VS Code), csvlens, VisiData, Excel | A grid with a sticky header and row numbers, sorting and a filter box. Rainbow CSV only colors each column in the raw text. Excel changes the data it opens (leading zeros dropped, values turned into dates), which is a reason to want a viewer that shows the file as written. |
| JSON | Firefox's built-in viewer, JSON Formatter extensions, Dadroit, jless and fx (terminal), JSON Crack (a graph) | A tree you can fold, colored by type, item counts on folded nodes, a filter, a tree / raw / pretty switch, copy value and copy path. |
| XML | Browsers' built-in tree, XML Notepad, QXmlEdit, XML Explorer, Firefox | A tree of elements, attributes inline, comments and CDATA, folding. The desktop tools are mostly old editors; there is no clean, current read-only XML viewer. |
| SVG | Browsers, PowerToys' preview pane and Peek, ImageGlass, VS Code SVG extensions | Fit to window, zoom, a checkerboard or switchable background, show source. |
| Anything | PowerToys Peek, QuickLook | One simple view per format and nothing else, which is the closest match to "a clean viewer for whatever file". |

### The ones worth copying

- **GitHub** shows `.csv` and `.tsv` as a table with a header and row numbers. The first row is always the header. Typing in the search box above the table filters its rows. A row or a range of rows can be linked (`#L6-L8`). It renders only files up to 512 KB ("anything bigger than that slows down the browser"), and a file whose rows have different numbers of separators shows an error instead of the table. It also renders Jupyter notebooks as static HTML, GeoJSON and TopoJSON as maps, STL as a 3D model, PDF, and Mermaid files.
- **Firefox's JSON viewer** parses and highlights a JSON file, shows arrays and objects collapsed with a control to expand each, has a search box that filters, a raw view and pretty printing. It reads JSON Lines (`.jsonl`) too: the document shows as an array with one collapsible entry per non-blank line, and pretty printing isn't offered for it.
- **jless** (a terminal JSON viewer, in Rust) has two views: data mode, a tree with folding objects and arrays, and line mode, the formatted text. The path of the focused node is always shown at the bottom, and copying it comes in three forms: JavaScript (`.foo[3].bar`), brackets (for Python) and jq. `c` and `e` fold and unfold the focused node and all its siblings, `C` and `E` do it deeply. Search is by regular expression over the whole text. It reads YAML into the same tree.
- **PowerToys Peek** previews Markdown, SVG, PDF, HTML and source code (in Monaco, the editor inside VS Code). A file whose extension it doesn't know is checked for text: text shows as plain text in the code view, anything else shows the file's details. It also shows SQLite databases read-only. PowerToys' File Explorer add-ons cover SVG, Markdown, PDF, G-code, QOI and over 150 code extensions in the preview pane, each in WebView2.
- **Tad** (MIT, Electron and DuckDB) opens CSV, TSV, semicolon-separated CSV as Excel writes it in many European locales, compressed CSV and Parquet, has a "no header row" option, and re-reads a file when its modified time changes. Its main view is a pivot table (filter, aggregate, sort, pick and order columns), which is analysis rather than viewing.
- **Modern CSV, CSV Buddy and Lazy CSV Viewer** open very large CSV files by reading only the page in view, with a memory footprint far smaller than the file.
- **Dadroit** shows JSON as a tree from the root, by type, and searches with regular expressions and JSONPath; it claims to open 1 GB files in seconds. Its free tier stops at 50 MB.

## 2. Simple features that don't edit

**CSV, TSV**

- Detect the delimiter (`,`, `;`, tab, `|`), handle quoted fields with line breaks in them, guess whether the first row is a header, with a switch.
- The header sticks (table-sorting/03), Sort (table-sorting/01), filter (table-sorting/02), Find.
- Numbers aligned right; a long cell cut short with the whole value in a tooltip; the rows and columns counted.
- Copy a cell or a row; copy a selection as TSV, so it pastes into Excel as cells.
- Maybe: a column's type, empty cells, distinct values, smallest and largest, on hovering its header.

**JSON**

- Fold and unfold, fold a node and all its siblings, unfold to depth N; item counts on folded nodes.
- The path of the node under the pointer or in focus, always shown, and copy path and copy value.
- The tree and the formatted text as two views of one document.
- An invalid file shows as highlighted text with the error's place marked.
- JSON with comments (JSONC, JSON5) is read, not refused.
- JSON Lines (`.jsonl`, `.ndjson`): one entry per line, as Firefox shows it; a table when every line is a flat object with the same keys.

**XML**

- The same tree: elements, attributes inline, text, comments, CDATA, prefixes shown as written.
- Copy the node's path (XPath).
- No DTD or external entity is ever resolved (XXE, "billion laughs").

**SVG**

- Fit to window, 100 %, zoom.
- A background switch (checkerboard, light, dark): an SVG drawn in black on a transparent background vanishes in the dark theme.
- The size and `viewBox`; show the source as highlighted XML.

**All of them:** Reload is the feature none of the simple viewers lead with but Downlight already has. A reader who writes an SVG with Graphviz or a CSV with a script gets a live preview for free (Tad's re-read on a changed modified time is the same idea).

## 3. How it fits Downlight

- **One entry point, chosen by format.** Spec story 83 wants rendering chosen by file format behind one entry point. `core/src/format.rs` would grow a `Format` (Markdown, text and code, CSV, SVG, JSON, XML) chosen by extension, and `render` dispatches on it.
- **Every format is still a rendered document.** The core turns each format into sanitized HTML whose blocks carry `data-line`. Find, anchoring a Reload, the reading position, Edit at the line, the Overview ruler and History then work unchanged.
- **SVG costs almost nothing.** `core/src/image.rs` already serves `.svg` as `image/svg+xml` through the image scheme, so an SVG document is a rendered document holding one `<img>` of its own file. The SVG specs make an image safe: an SVG an HTML `img` references is processed in secure animated mode, with scripts disabled, no external file loaded and no interactivity, while declarative animation (SMIL, CSS) still plays; content in a `foreignObject` gets the same limits. The cost: an SVG that loads its own fonts or images from outside shows without them.
- **CSV is a table, but not one block.** ADR 0003 gives each top-level block `content-visibility: auto`, so the webview lays out only the blocks near the view. A CSV as a single `<table>` is one block, and a 100 MB one would be laid out whole. The rows need to come in many top-level blocks (a table per few hundred rows sharing column widths, or `tbody`s if they can be blocks), each carrying its first record's line. Records, not lines: a quoted field can span lines, so a record carries the line it starts on. The fallback, if that measures too slow, is a grid that draws only the rows in view, as Modern CSV and CSV Buddy do; it breaks "the core renders HTML" and needs new commands between the page and the app, so it comes only after a measurement.
- **JSON and XML trees as nested `<details><summary>`**, made by the core, fold with no script. Browsers' own find in page opens a closed `<details>` that holds a match (Chromium since 97, Firefox, Safari), and `hidden="until-found"` with its `beforematch` event is the general form, rendered with `content-visibility: hidden`. Downlight's Find is its own (`page/src/find.ts`, CSS highlights), so it doesn't get this for free: it opens the current match's ancestors itself. The top level's entries are the document's blocks, for ADR 0003.
- **Build the tree from the file's tokens, not a parsed value.** That keeps each value's line, key order, duplicate keys and numbers as written (JavaScript's `JSON.parse` loses precision past 2^53, so a long ID would show wrong). Candidate crates, to check for license and upkeep before choosing: `json-syntax` (strict RFC 8259 parser on `locspan`, a position for every value, entries in order, duplicates kept, numbers kept as text, a lenient mode), `spanned_json_parser` (line and column on every value, works with serde; order and duplicates not stated), `hifijson` (no dependencies, keeps the input exact, especially numbers). For CSV, the `csv` crate; for XML, `quick-xml`.
- **Code and plain text first.** Since ticket 34, any text file Downlight opens renders as Markdown, so a `.py` file's `#` comments become headings. Showing it as highlighted text (Peek's fallback) reuses the highlighting of ADR 0004 and sets up the format choice the other tickets need.
- **"Open with", never default.** The installer lists Downlight in "Open with" for each new extension and claims none of them.
- **The glossary.** **Document** loses "rendered as Markdown", and **Format** may become a term. Neither changes until a ticket is grilled.

## 4. File types beyond the tickets

| Kind | Extensions | How Downlight would show it |
| --- | --- | --- |
| Code and plain text | `.txt`, `.log`, `.rs`, `.py`, `Dockerfile`, `Makefile`, `.gitignore`, `.editorconfig`, `LICENSE` | Highlighted text, by extension or file name (ticket 03). |
| Configuration | `.ini`, `.cfg`, `.conf`, `.toml`, `.yaml`, `.env`, `.properties`, `.reg` | Highlighted text, its sections or top-level keys in the outline (ticket 03); a YAML tree could reuse ticket 02's later, as jless does. |
| Patches | `.diff`, `.patch` | Highlighted text, with added and removed lines colored as GitHub colors them (ticket 03). |
| Notebooks | `.ipynb` | Markdown cells, code cells and their text and image outputs, as GitHub shows them (ticket 04). |
| JSON-based | `.jsonl`, `.ndjson`, `.json5`, `.jsonc`, `.har`, `.geojson` | As JSON (ticket 02). No map for GeoJSON: maps need tiles from the network. |
| XML-based | `.csproj`, `.xaml`, `.resx`, `.plist`, `.xlf`, RSS and Atom | As the XML tree (ticket 02). A feed shown as articles would be another product. |
| Diagram sources | `.mmd`, `.mermaid` | With [mermaid-and-math/01](../mermaid-and-math/issues/01-mermaid-and-math.md). `.dot` and `.puml` need layout engines of their own: out. |
| Others | `.pdf`; `.stl`; SQLite; `.srt`, `.vtt`, `.ics`, `.vcf` | PDF belongs with [converter/01](../converter/issues/01-word-pdf-and-powerpoint-converter.md); STL needs a 3D viewer and SQLite is binary: out. Subtitles and contact or calendar files are niche: text (ticket 03) is enough. |

**Editing is out**, even for configuration files. The glossary's Downlight is "reading only; changing a file happens in another program", and Edit already opens the file in an editor at the reading position's line. Editing in the window brings saving, undo, keeping the encoding and line endings, and a clash with watching and Reload: another product. [writing-ticks/01](../writing-ticks/issues/01-writing-ticks-to-the-file.md) is the one write path proposed, and it is already a hard case.

## Sources

- [GitHub Docs: Working with non-code files](https://docs.github.com/en/repositories/working-with-files/using-files/working-with-non-code-files)
- [GitHub Docs: Rendering CSV and TSV data](https://docs.github.com/en/github/managing-files-in-a-repository/working-with-non-code-files/rendering-csv-and-tsv-data)
- [Sarah Ford: CSV and TSV as an interactive table on GitHub](https://saraford.net/2017/02/25/how-to-render-a-csv-and-tsv-file-as-an-interactive-table-on-github-056/)
- [Firefox Source Docs: JSON viewer](https://firefox-source-docs.mozilla.org/devtools-user/json_viewer/)
- [jless](https://jless.io/) and [its releases](https://github.com/PaulJuliusMartinez/jless/releases)
- [PowerToys Peek](https://learn.microsoft.com/en-us/windows/powertoys/peek)
- [PowerToys File Explorer add-ons](https://learn.microsoft.com/en-us/windows/powertoys/file-explorer)
- [Tad](https://www.tadviewer.com/) and [Tad 0.10.0 on DuckDB's discussions](https://github.com/duckdb/duckdb/discussions/3566)
- [Modern CSV](https://www.moderncsv.com/), [CSV Buddy](https://www.csv-buddy.com/), [Lazy CSV Viewer](https://github.com/Rudra370/lazy_csv_viewer)
- [Best CSV viewers and readers 2026 (importcsv)](https://www.importcsv.com/blog/csv-viewer-reader)
- [Dadroit: how to open large JSON files](https://dadroit.com/blog/open-big-json/) and [Dadroit JSON Viewer on AlternativeTo](https://alternativeto.net/software/dadroit-json-viewer/about)
- [Open source XML viewers for Windows (listoffreeware)](https://listoffreeware.com/free-open-source-xml-viewer-software-windows/), [XML Notepad alternatives (AlternativeTo)](https://alternativeto.net/software/xml-notepad/?license=free)
- [SVG 2: Conformance Criteria](https://www.w3.org/TR/SVG2/conform.html), [SVG 2: Embedded Content](https://www.w3.org/TR/SVG2/embedded.html), [SVG Integration](https://svgwg.org/specs/integration/), [W3C wiki: SVG/Animation](https://www.w3.org/wiki/SVG/Animation), [Mozilla bug 628747](https://bugzilla.mozilla.org/show_bug.cgi?id=628747)
- [Chrome for Developers: hidden=until-found](https://developer.chrome.com/docs/css-ui/hidden-until-found), [Interop issue 491](https://github.com/web-platform-tests/interop/issues/491)
- [json-syntax](https://crates.io/crates/json-syntax), [spanned_json_parser](https://docs.rs/spanned_json_parser/latest/spanned_json_parser/), [hifijson](https://crates.io/crates/hifijson)
