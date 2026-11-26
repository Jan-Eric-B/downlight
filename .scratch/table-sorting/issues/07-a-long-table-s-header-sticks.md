# 07: A long table's header sticks to the top

**What to build:** While the reader scrolls through a long table, its header row stays at the top of the view, so the columns stay named.

Spec: none yet. Grill it first (`/grill-with-docs`). From the Sort grilling (2026-11-19, recorded in the [spec](../spec.md)), which put it on Later.

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Which tables: every table taller than the view, or only Markdown tables (raw HTML tables get no sort button either)?
  - github-markdown-css shows a wide table in its own horizontal scroller, which breaks a sticky header; how a wide table behaves.
  - The find bar and the file-gone bar at the top of the view: the header sticks below them.
  - The **Reading position** is the block at the top of the view: a stuck header must not count as it.

## Comments

**2026-11-20.** From the Sort grilling (2026-11-19): "a header that sticks to the top of a long table go[es] on Later."
