# 01: Marp slideshow

**What to build:** A document written as Marp slides (front matter with `marp: true`, slides split by `---`) can be shown as a slideshow: one slide at a time filling the window, with arrow keys, Page Up and Page Down, Space, Home and End to move, and Esc to leave.

Spec: none yet. Grill it first (`/grill-with-docs`).

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - How much of Marp: only splitting and paging, as mdhero does ("minimal"), or Marp's directives (`paginate`, `theme`, `header`, `footer`, background images)? Marp's own renderer (Marp Core, MIT) is a JavaScript dependency; Downlight renders in Rust (ADR 0002), so either the core learns the split or the page loads Marp only for such documents.
  - How it starts: a title bar button that shows only for Marp documents, a shortcut (F5 is Reload today), or automatically?
  - Its relation to full screen (`.scratch/full-screen/issues/01-full-screen.md`): a slideshow is usually shown full screen.
  - What a **Reload** does mid-show: stay on the same slide number. What **Find**, **History** and Edit do during a show.
  - Front matter shows today as a YAML code block; in a show it is hidden.
  - Its glossary term ("Slideshow"?).

## Comments

**2026-11-20, from the repo owner.** Picked from the mdhero comparison (`.scratch/mdhero-comparison/features.md`, "Marp slides [partly]"). The owner picked it against the recommendation to skip it as niche.
