# 01: Outline, the document's headings to jump to

**What to build:** A list of the shown document's headings, built from its rendered headings, with the heading in view marked as the reader scrolls. Choosing one scrolls to it, as a heading link does. It covers the v1 spec's "Outline" (v1.x) and "a table of contents sidebar, after Outline" (Later) as one list with two placements.

Spec: none yet. Grill it first (`/grill-with-docs`), then `/to-spec` writes `.scratch/outline/spec.md`.

**Blocked by:** none

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - **Where it goes.** The candidates, with the recommendation first:
    1. **A flyout under a title bar button and a shortcut (Ctrl+Shift+O, VS Code's "Go to symbol")**, as the settings flyout opens under the gear: it lies over the document, so nothing reflows and the reading position stays, and it works at any window width and with Full width. Typing filters the headings; Enter or a click goes to one. A right-click menu entry opens the same flyout.
    2. **Docked in the left margin** when the window is wider than the reading width plus room for the list (about 240 px each side), shown or hidden by the same button. When the window gets too narrow, or Full width is on, it falls back to the flyout rather than narrowing the document, since narrowing rewraps the text on every toggle.
    3. **The current heading beside the file name in the title bar** (VS Code's breadcrumbs), its click opening the flyout at that heading. Cheap and always visible, but the title bar is already full in a narrow window.
    - Rejected: **a hover menu on the file name** (hover is undiscoverable and has no keyboard path, and the file name's tooltip and click already mean the path, the counts and revealing the file) and **the headings as a native right-click submenu** (no filter, no marked heading, nesting h2 and h3 in submenus, and hundreds of entries in a large document).
  - Which heading levels; whether choosing a heading joins **History** as a heading link does.
  - **Heading jump keys:** keys that go to the next or previous heading (mdhero uses `[` and `]`; Alt+Down and Alt+Up are another choice), from the reading position. Which keys, which levels, and whether a jump joins History. They need only the list of headings, so they can ship as a first slice ahead of the flyout.
  - Large documents: how many headings it must handle (the 100 MB test document) and whether it waits for ticket 39 of the v1 spec.
  - Its name in the glossary: "Outline" (the spec's word) vs "Table of contents"; option C of the overview ruler's mockups is this list squeezed into the ruler, and was dropped for that.

## Comments

**2026-11-20, from the repo owner.** Asked for after the comparison with mdhero (`.scratch/mdhero-comparison/features.md`, its table of contents sidebar). Kept apart from the **Overview ruler** (`.scratch/overview-ruler/research.md`): the two share only the list of headings.
