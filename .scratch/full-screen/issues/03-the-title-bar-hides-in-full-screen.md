# 03: The title bar hides in full screen

**What to build:** While the window is **Full screen** the **Title bar** is hidden and the document starts at the top of the screen, with the Find bar and the settings flyout moved up to the top too. The reading position stays when the window goes full screen and when it leaves. The bar doesn't slide in yet (ticket 04): F11, Esc and the right-click menu leave full screen.

Spec: [Full screen](../spec.md)

**Blocked by:** 01 (F11 and the menu switch full screen)

**Status:** ready-for-agent

- [ ] Prefactor first: today one variable is both the bar's own height and the room the page keeps for the bar at the top (the view, the Find bar, the settings flyout). Separate them, without changing what existing tests or the window show.
- [ ] While full screen, the room kept for the bar is zero and the bar is hidden; out of full screen both are as today. On systems without the page's title bar (the native one) nothing changes.
- [ ] The Notices, the Counts bubble, the Overview ruler, the Wait note and the file deleted or moved bar stay where they are; the window's own title doesn't change.
- [ ] The view keeps the reading position across going full screen and leaving it, as for any layout change that keeps it.
- [ ] Page tests (written first): in full screen the bar is hidden and the room kept for it is zero; leaving brings both back; the bar's state follows the app's answer, not the page's expectation.
- [ ] Hand check in the release build: in full screen the document starts at the top of the screen; Find (Ctrl+F) and the settings flyout (Ctrl+,) open at the top; the reading position stays on entering and leaving, on a long document.
- [ ] `npm run check` passes.
