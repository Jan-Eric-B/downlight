# 01: Reader settings

**What to build:** The reader can change how the document's text looks: its font, size and line height, beyond zoom.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later ("reader settings").

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Which choices: font family (system, serif, the reader's own), size beside zoom, line height, paragraph spacing, wrapping code? Each one moves away from GitHub's look, which is the point of Downlight's.
  - **Wrapping long code lines** (the owner wants it): a switch that wraps code blocks' long lines instead of scrolling them sideways. Does it apply to every code block or only ones wider than the column, and does the **Highlight limit**'s plain code wrap too?
  - Not the reading width: the owner dropped a width of the reader's own (`.scratch/reading-width/issues/01-reading-width.md`, wontfix); GitHub's reading width and **Full width** stay.
  - Where they live: the settings flyout's "View" group (from the **Overview ruler**), or a reader panel of their own as mdhero has.
  - Every change keeps the **Reading position**, as zoom and **Full width** do.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). mdhero has reader controls, including width and code wrapping (`.scratch/mdhero-comparison/features.md`). Not yet grilled.
