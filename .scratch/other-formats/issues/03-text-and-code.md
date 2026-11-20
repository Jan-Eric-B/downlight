# 03: Code and plain text as highlighted text

**What to build:** Downlight shows a text file that isn't Markdown (source code, a log, a configuration file, a patch) as its text, highlighted by its language, instead of rendering it as Markdown. Today, since ticket v1/34, any text file Downlight opens renders as Markdown, so a `.py` file's `#` comments become headings and its `*` turn text italic. Downlight shows up in "Open with" for none of these unless the grilling decides otherwise.

Spec: none yet. Grill it first (`/grill-with-docs`). Came from the research for tickets 01 and 02: [research.md](../research.md).

**Blocked by:** None (can start immediately). It sets up rendering chosen by format (the v1 spec's story 83), which 01 and 02 need, and gives 02 its raw view and its view of an invalid file.

**Status:** needs-triage

- [ ] Grill it before planning. Open questions, with what the research found:
  - Which files are Markdown and which text: by extension and by file name (`Dockerfile`, `Makefile`, `.gitignore`, `LICENSE`). A file with an unknown extension or none (`README`): Markdown, as today, or text, as PowerToys Peek shows any text file it doesn't know?
  - `.txt`: plain text, or Markdown, as many `.txt` notes are written?
  - Highlighting with the grammars and GitHub's classes the core already uses (ADR 0004), the language found by extension or file name. The **Highlight limit** applies: a large log shows as plain text.
  - Large files: one `<pre>` is one block, and ADR 0003 lays out only top-level blocks near the view, so the text comes in blocks of some hundreds of lines, each carrying its first line, for the reading position, Reload's anchoring and Edit.
  - Patches (`.diff`, `.patch`): added and removed lines colored as GitHub colors them, or only the diff grammar's highlighting?
  - Configuration files (`.ini`, `.cfg`, `.conf`, `.toml`, `.yaml`, `.env`, `.properties`, `.reg`): their sections or top-level keys as headings for the Outline (outline spec) and the **Overview ruler**?
  - Editing stays out, for configuration files too: Downlight is reading only, and Edit opens the file in an **Editor** at the reading position's line. Editing in the window would bring saving, undo, keeping the encoding and line endings, and a clash with watching and **Reload** (research.md, section 4).
  - Long lines: wrapped or scrolled sideways (reader-settings/01 has wrapping long code lines)?
  - Which features apply: Find, **Reload** (a log that grows), line numbers in the margin (line-numbers/01, which fits code better than Markdown), the **Counts**.
  - The glossary: **Document** is "rendered as Markdown"; this ticket or 01, whichever comes first, widens it.

## Comments

**2026-11-20.** Stub from the research for tickets 01 and 02 ([research.md](../research.md)): PowerToys Peek shows a text file it doesn't know as plain text in a code view, while Downlight shows it as Markdown. Recommended before 01, as it sets up the format choice cheaply.
