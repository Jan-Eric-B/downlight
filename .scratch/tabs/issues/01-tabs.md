# 01: Tabs

**What to build:** Several documents in one window as tabs, as an alternative to a window per document.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later.

**Blocked by:** None (can start immediately)

**Status:** wontfix

- [ ] Grill it before planning. Open questions:
  - ADR 0001 decides one process with a window per document. Do tabs replace that, or sit beside it as an option? Either needs that ADR amended or superseded.
  - What becomes per tab: **History**, the **Showing**, **Ticks**, **Sort**, Find, **Keep on top**, the **Active window**?
  - Where the tabs go with the page-drawn **Title bar** (ADR 0005): in the title bar as browsers do, or a row below?
  - Where a document opened from the file manager goes: a new tab in the active window, or a new window?
  - Memory and start time with many tabs open.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). mdhero has tabs on top or on the side (`.scratch/mdhero-comparison/features.md`). Not yet grilled.

**2026-11-20, from the repo owner.** Dropped (wontfix). Downlight keeps one window per document (ADR 0001), and the v1 spec lists tabs under Out.
