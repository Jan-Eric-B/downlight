# 06: Blocks shaded by kind

**What to build:** Under its marks, the **Overview ruler** tints each row by the kind of block there (code, tables, images, quotes and alerts, lists), so the reader sees the document's shape: "the code is in the second half". Option B of the research and mockups, an optional layer.

Spec: [Overview ruler](../spec.md), which puts it out of scope until this is grilled.

**Blocked by:** 05 (Large documents, measured)

**Status:** needs-triage

- [ ] Grill it after ticket 05's measurement. Open questions:
  - Whether it is worth the noise beside the scroll bar, once the marks-only ruler has been used for a while.
  - A switch of its own in the flyout's "View" group, or part of the ruler's switch? Its default.
  - Five or six tints that work in light and dark, on Mica and with the **Accent color**; off in high contrast, where only a few system colors exist?
  - The research's cost: a binary search per pixel row for the block there, so it scales to 100 MB; measured as ticket 05 measures.

## Comments

**2026-11-20.** From the overview ruler's grilling: kept out of the spec as a later, optional layer (`.scratch/overview-ruler/research.md`, option B; `mockups-light.png`).
