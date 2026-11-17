# 14: The app's name beside the logo in the empty window

**What to build:** A window that shows no document shows "Downlight" in the title bar next to the logo, where the file name stands once a document is shown. As soon as the window shows a document, the file name takes its place.

Spec: [Window design](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] The empty window shows "Downlight" right of the logo, in the file name's place and caption font (12 px Segoe UI Variable), dimmed with the bar when the window isn't active.
- [ ] It is plain text, not a button: no hover look, no tooltip, no click (there is no file to reveal), and it is part of the drag region, as the rest of the empty bar is.
- [ ] Showing a document replaces it with the file name; nothing shows both. The zoom level stays hidden in the empty window, as today.
- [ ] The window's OS title in the empty window is unchanged (v1 story 10).
- [ ] Page tests: the empty window shows the name; after a show the file name replaces it.
- [ ] Checked by hand: an empty window (Downlight started with no file) in light, dark and a contrast theme, active and inactive.

## Comments

**2026-11-17, from the repo owner.** Asked for as a feature idea. Spec story 1 keeps the name off the bar "without its name taking space", which is about a window that shows a document: the empty window has nothing else in that place, so the name costs no space there. Ticket 04 decided "The empty window shows only the logo" (`page/src/title-bar.ts`, the file name element and the zoom level's hidden rule); this ticket changes that one sentence of ticket 04's.
