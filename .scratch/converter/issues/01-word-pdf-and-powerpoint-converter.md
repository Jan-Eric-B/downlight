# 01: A Word, PDF and PowerPoint converter

**What to build:** A separate, optional component that turns a Word, PDF or PowerPoint file into Markdown, so Downlight can show it.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later ("as a separate optional component").

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - What it is: a second installer, a download, or a plugin Downlight finds? Downlight makes no network requests of its own, so it never fetches it by itself.
  - Which converter (for example Microsoft's MarkItDown, in Python), its license against GPL-3.0, and its size.
  - Whether the result is saved as a `.md` file beside the original, or shown without a file (the same question as pasting Markdown text, `.scratch/paste-markdown/`).
  - Untrusted input: the converter runs out of process, with a time limit, and its output goes through the core's sanitizing like any **Document**.
  - **Reload**: does a change to the original convert again?

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). Not yet grilled.
