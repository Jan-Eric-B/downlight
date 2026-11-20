# 02: JSON and XML as collapsible trees

**What to build:** Downlight shows a JSON or XML file as a tree the reader can fold and unfold. Downlight shows up in "Open with" for them but never asks to be their default app.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later. Research: [research.md](../research.md).

**Blocked by:** 01 (CSV and TSV as a table, SVG as an image), which sets up rendering by format, or 03 (text and code), whichever builds it first.

**Status:** needs-triage

- [ ] Grill it before planning. Open questions, with what the research found:
  - The tree as nested `<details><summary>` that the core makes, so folding needs no script. The top level's entries are the document's blocks, for ADR 0003.
  - Built from the file's tokens, not a parsed value, so each value keeps its line, keys keep their order, duplicate keys stay and numbers show as written (JavaScript's `JSON.parse` loses precision past 2^53). Candidate crates, to check for license and upkeep: `json-syntax` (a position for every value, entries in order, duplicates kept, numbers as text, a lenient mode), `spanned_json_parser` (line and column on every value), `hifijson`; `quick-xml` for XML.
  - An invalid file: shown as plain highlighted text (ticket 03's view) with the error's place marked, or refused with a **Notice**? JSONView shows the raw text; Firefox shows its error.
  - JSON with comments (JSONC, JSON5): read, or shown as text?
  - JSON Lines (`.jsonl`, `.ndjson`): one collapsible entry per non-blank line, as Firefox shows it, and a table (ticket 01's) when every line is a flat object with the same keys?
  - The views: the tree and the formatted text, as jless's data and line modes and Firefox's JSON and Raw Data. The raw view is the file itself, shown by ticket 03.
  - Folding commands: fold and unfold one node, fold a node and all its siblings (jless's `c` and `e`, the one that matters in a long array), unfold to depth N. Item counts on a folded node ("[12]").
  - The path of the node in focus, always shown, and copying it: JSON as JavaScript (`.foo[3].bar`) or jq, as jless offers; XML as XPath. Copy value. Where does the path show, given the title bar and the selection's **Counts** bubble?
  - Large files: folded below some depth by default, and only the open parts laid out, for a 100 MB JSON. Dadroit claims 1 GB in seconds (its own claim).
  - XML specifics: attributes inline, prefixes shown as written with no namespace lookup, comments, CDATA, and never resolving a DTD or external entity (XXE, "billion laughs").
  - Find inside folded nodes: browsers' own find in page opens a closed `<details>` holding a match, but Downlight's Find is its own (CSS highlights), so it opens the **Current match**'s ancestors itself. A JSONPath search, as Dadroit has, belongs with find-options/01.
  - Whether a fold belongs to the showing, as **Ticks** do, and survives a **Reload**.
  - Which extensions: `.json`, `.xml`, and those built on them (`.jsonl`, `.ndjson`, `.json5`, `.jsonc`, `.har`, `.geojson` as JSON, with no map since maps need tiles from the network; `.csproj`, `.xaml`, `.resx`, `.plist`, `.xlf`, RSS and Atom as XML). Which get "Open with" entries?
  - YAML in the same tree, as jless reads it, here or later?

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later), after CSV, TSV and SVG.

**2026-11-20.** Open questions extended from a web search of other viewers (Firefox's JSON viewer, jless, Dadroit, XML Notepad) and of Rust JSON parsers; the findings and sources are in [research.md](../research.md). Jupyter notebooks, JSON underneath, are ticket 04.
