# 14: The app's name beside the logo in the empty window

**What to build:** A window that shows no document shows "Downlight" in the title bar next to the logo, where the file name stands once a document is shown. As soon as the window shows a document, the file name takes its place.

Spec: [Window design](../spec.md)

**Blocked by:** none

**Status:** ready-for-human

- [x] The empty window shows "Downlight" right of the logo, in the file name's place and caption font (12 px Segoe UI Variable), dimmed with the bar when the window isn't active.
- [x] It is plain text, not a button: no hover look, no tooltip, no click (there is no file to reveal), and it is part of the drag region, as the rest of the empty bar is.
- [x] Showing a document replaces it with the file name; nothing shows both. The zoom level stays hidden in the empty window, as today.
- [x] The window's OS title in the empty window is unchanged (v1 story 10).
- [x] Page tests: the empty window shows the name; after a show the file name replaces it.
- [ ] Checked by hand: an empty window (Downlight started with no file) in light, dark and a contrast theme, active and inactive.

## Comments

**2026-11-17, from the repo owner.** Asked for as a feature idea. Spec story 1 keeps the name off the bar "without its name taking space", which is about a window that shows a document: the empty window has nothing else in that place, so the name costs no space there. Ticket 04 decided "The empty window shows only the logo" (`page/src/title-bar.ts`, the file name element and the zoom level's hidden rule); this ticket changes that one sentence of ticket 04's.

**2026-11-17, built; the hand check is left.** Only the page changed:

- **The name is a span of its own**, `createAppName` in `page/src/title-bar.ts`, between the logo and the file name, rather than the file name's button showing other text: a button would be in the accessibility tree as one, and would have to be taken out of the drag region and back. As a span with no `bar-item` class it has no hover, no tooltip, no click, and the bar's `app-region: drag` covers it. It inherits the bar's color, so it dims with it, and `CanvasText` or `GrayText` in high contrast. In `page/src/title-bar.css` it shares the file name's caption font, margin and padding (`.title-bar :is(.app-name, .file-name, .zoom-level)`), so its text starts where the file name's does.
- **It shows only once the view says the window is empty** (the `whenDocumentShown` listener with no file), and starts hidden, so a window opened on a document never shows "Downlight" before its file name arrives, also not during a wait note. A window whose app couldn't be asked also hears no file, and shows the name too, as it has no file name to show.
- **The window's OS title** comes from `title_of` in `app/src/documents/show.rs`, which already gives "Downlight" for an empty window; the app isn't touched.
- **Tests** in `page/src/title-bar.test.ts` (the first written first, failing): the empty window shows "Downlight" right of the logo and no file name; a show replaces it with the file name and an empty window brings it back; a window opened on a document never shows it, also before its document arrives; it is no button, has no tooltip, isn't a bar item, and a click asks the app nothing. They replace ticket 04's "the empty window shows only the logo". The zoom level's existing test for the empty window stays. `titleBarAppName` joins the title bar helpers in `page/src/test-support.ts`.
- Ticket 04's box and the spec's title bar section now say the empty window shows "Downlight" (this ticket).
