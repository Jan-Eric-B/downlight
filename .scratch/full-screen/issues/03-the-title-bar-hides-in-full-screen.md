# 03: The title bar hides in full screen

**What to build:** While the window is **Full screen** the **Title bar** is hidden and the document starts at the top of the screen, with the Find bar and the settings flyout moved up to the top too. The reading position stays when the window goes full screen and when it leaves. The bar doesn't slide in yet (ticket 04): F11, Esc and the right-click menu leave full screen.

Spec: [Full screen](../spec.md)

**Blocked by:** 01 (F11 and the menu switch full screen)

**Status:** built, hand checks left

- [x] Prefactor first: today one variable is both the bar's own height and the room the page keeps for the bar at the top (the view, the Find bar, the settings flyout). Separate them, without changing what existing tests or the window show.
- [x] While full screen, the room kept for the bar is zero and the bar is hidden; out of full screen both are as today. On systems without the page's title bar (the native one) nothing changes.
- [x] The Notices, the Counts bubble, the Overview ruler, the Wait note and the file deleted or moved bar stay where they are; the window's own title doesn't change.
- [x] The view keeps the reading position across going full screen and leaving it, as for any layout change that keeps it.
- [x] Page tests (written first): in full screen the bar is hidden and the room kept for it is zero; leaving brings both back; the bar's state follows the app's answer, not the page's expectation.
- [ ] Hand check in the release build: in full screen the document starts at the top of the screen; Find (Ctrl+F) and the settings flyout (Ctrl+,) open at the top; the reading position stays on entering and leaving, on a long document.
- [x] `npm run check` passes.

## Comments

**2026-11-20, built; the hand check is left.** What was built, and what it decided:

- **Prefactor: the bar's height and the room kept for it are two variables.** `title-bar.css` keeps `--title-bar-height` (0px, 32px while the page draws the bar) for the bar's own height, and adds `--title-bar-room`, which defaults to `var(--title-bar-height)`. The view's scroller (`document.css`), the Find bar (`document.css`) and the settings flyout (`settings-flyout.css`, its `top` and `max-height`) now start below `--title-bar-room`. The page's tests passed unchanged after it.
- **A hook for full screen's changes.** `whenFullScreenChanges(listener)` in `page/src/full-screen.ts` runs the listener with the new state each time an app answer changes it (a failed switch, an answer for an earlier start, and an answer equal to the state hear nothing), and returns a function that removes it. It hears nothing at the start: a new window's page starts out of full screen everywhere. Chosen over following `refreshCommands`, which is for the command buttons' `disabled` and `aria-pressed`: the bar's layout isn't a command button's state. Tickets 04 and 05 hook in here too.
- **The title bar follows it (`startTitleBar`, `page/src/title-bar.ts`).** In full screen the bar gets the class `away` and the page sets `--title-bar-room` to `0px` inline on the root; leaving removes both, so the stylesheet keeps the room again. On systems with the native title bar `startTitleBar` returns before it listens, so nothing changes there. `.title-bar.away` is `visibility: hidden` and `app-region: no-drag`: it takes no clicks, no focus and drags nothing, so the top 32 px are the document's, and it stays in the layout, so the settings flyout still lines up with the gear. Nothing touches the window's own title; the Notices are fixed at the bottom, and the Wait note and the file deleted or moved bar are sticky in the scroller, so they sit at the top of the document as before. The Counts bubble and the Overview ruler aren't built yet.
- **The reading position.** No code kept it across a window resize before: a plain resize relies on Chromium's scroll anchoring in the scroller. The change of room now goes through the view's `keepReadingPosition`, as zoom and Full width do, so the view reads the reading position, the room and the bar change, and the view puts the block back at the top after the next frame. The window's own resize may reach the webview before or after the app's answer; scroll anchoring covers what lands after that frame. The hand check on a long document confirms it.
- **Tests** (written first, failing): `page/src/title-bar.test.ts`: `a new window's title bar shows, with its room kept`; `in full screen the title bar is hidden and the page keeps no room for it`; `leaving full screen shows the title bar again and keeps its room again`; `the title bar shows the full screen the app answers with, not the one the page expected`; `a full screen the app couldn't switch leaves the title bar and its room as they were`; `going full screen and leaving keep the reading position, with the room for the bar already changed`; `without the page's title bar, full screen leaves the room at the top as it is`. New helpers in `page/src/test-support.ts`: `titleBarShows()` and `titleBarRoom()`. Changed: `title-bar.test.ts`'s fake app answers `switch_full_screen`, its `beforeEach` starts full screen, its `afterEach` removes the room, and `shownFrom` takes the rendered document.
- **For tickets 04 and 05.** 04 decides `away` from full screen and the pointer, focus and flyout, and keeps the room at zero; it must replace `visibility: hidden` (a hidden bar can't take the focus, so Tab couldn't reach it) with a slide, e.g. a transform. While the bar is away the app still answers the pointer at the maximize button's place as the system's maximize button (snap layouts, maximize on a click) at the screen's top right: ticket 05's change to the app fixes that.
- **Not done:** the hand check in the release build.
