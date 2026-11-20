# 01: Find options

**What to build:** **Find** can match case, whole words only, or a regular expression, switched in the find bar.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later.

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Find indexes a case-folded copy of the document's text (v1 ticket 14). Match case needs the unfolded text too: index both, or rebuild on switch?
  - Regex: whose syntax (the page's JavaScript `RegExp`), a match across inline formatting as today, and guarding the 100 ms budget against a slow pattern on the 1 MB document.
  - Whether the options are kept in the **Settings** or belong to one window.
  - How they show in the find bar (toggle buttons with Alt+C, Alt+W, Alt+R as VS Code does?).
  - The **Overview ruler**'s marks follow whatever Find matches.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). Not yet grilled.
