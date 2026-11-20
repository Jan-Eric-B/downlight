# 04: Jupyter notebooks

**What to build:** Downlight shows a Jupyter notebook (`.ipynb`) as GitHub shows one: its Markdown cells rendered, its code cells highlighted, and their text and image outputs below them. Downlight shows up in "Open with" for `.ipynb` but never asks to be its default app.

Spec: none yet. Grill it first (`/grill-with-docs`). Came from the research for tickets 01 and 02: [research.md](../research.md). GitHub renders `.ipynb` files as static HTML (GitHub Docs, "Working with non-code files").

**Blocked by:** 02 (JSON and XML as collapsible trees), for its JSON parsing with lines; the notebook's view needs none of the tree.

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - A notebook is JSON: its cells' text and outputs are strings inside it, so a cell's line in the file is the line of its JSON entry, not of its Markdown. What the reading position, Reload's anchoring and Edit use.
  - Outputs are untrusted: HTML outputs through the core's sanitizing like a document's HTML, images (base64 PNG, JPEG or SVG) as images, and scripts, widgets and JavaScript outputs never run.
  - Which output kinds: plain text, errors with their tracebacks (ANSI colors), images, HTML tables (pandas), Markdown and LaTeX outputs (mermaid-and-math/01 for the math).
  - Execution counts (`In [3]`) and cell boundaries: shown, and how quietly?
  - Large notebooks: outputs with large embedded images, and the **Highlight limit** across cells.

## Comments

**2026-11-20.** Stub from the research for tickets 01 and 02 ([research.md](../research.md)). A notebook is mostly Markdown and code, so it fits Downlight better than any other format beyond Markdown.
