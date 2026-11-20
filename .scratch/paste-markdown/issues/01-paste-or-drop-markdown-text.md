# 01: Paste or drop Markdown text

**What to build:** The reader can paste Markdown text (Ctrl+V), or drop text dragged from another program, and Downlight shows it rendered, without saving it to a file first.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, v1.x.

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - The glossary's **Document** is a file. Text with no file has no path, so no watching or **Reload**, no Edit, no folder for relative links and **Local images**. Is it a new concept with its own term, and what does each of those do for it?
  - Where it shows: in the empty window only, or in a new window from any window? Does a drop of text over a shown document replace it, as a dropped file does?
  - What the title bar shows in place of the file name, and whether it joins **History**.
  - Limits: the largest paste, and binary-looking content (the **Binary file** rule is about files).
  - The v1 spec puts "reading from stdin or `--text`" under Out: does this change that?

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (v1.x). mdhero has "Paste Markdown" in its File menu (`.scratch/mdhero-comparison/features.md`). Not yet grilled.
