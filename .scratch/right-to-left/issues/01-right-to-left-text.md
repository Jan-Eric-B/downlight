# 01: Right-to-left text

**What to build:** Each block of a document takes its direction from its own text, as GitHub does, so an Arabic, Hebrew or Persian paragraph, heading, list item or table cell runs right to left and aligns right, while English blocks in the same document stay left to right.

Spec: none yet. Grill it first (`/grill-with-docs`), though it may be small enough to go straight to a ticket.

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Which elements get `dir="auto"`: GitHub puts it on paragraphs, headings, list items, quotes and table cells; check what GitHub does today and match it. The core adds it while rendering (the sanitizer already lets a document's own `dir` through, but nothing adds one).
  - Code blocks stay left to right whatever their content.
  - What follows the direction: task list checkboxes, the copy button's corner, alerts' bars, the **Line number** margin and the **Overview ruler** (which stays on the right).
  - **Find**, the **Reading position** and **Ticks** work the same in right-to-left blocks: tests with a right-to-left fixture.
  - The rendered fixtures change (`npm run fixtures`), since every block gains the attribute.

## Comments

**2026-11-20, from the repo owner.** Picked from the mdhero comparison (`.scratch/mdhero-comparison/features.md`, "Right-to-left blocks"): mdhero puts `dir="auto"` on every text block. Not in Downlight's specs so far.
