# 01: Spike: the page draws the title bar

**What to build:** The window's native frame goes, and the page draws a plain title bar: an empty drag area and minimize, maximize and close. Everything the native title bar did still works, the snap layouts flyout above all. This ticket proves ADR 0005 before anything is built on it; if snap layouts can't be brought back, stop here and record why.

Spec: [Window design](../spec.md), ADR [0005](../../../docs/adr/0005-the-page-draws-the-title-bar.md)

**Blocked by:** none

**Status:** ready-for-human

- [x] Windows open with `decorations: false`. The page draws a title bar of Windows 11's height (32 px) with minimize, maximize (restore when maximized) and close, using Fluent UI System Icons inlined as SVG, with their MIT notice.
- [ ] Hovering maximize shows Windows' snap layouts flyout. The Windows module answers `WM_NCHITTEST` with `HTMAXBUTTON` over the page's maximize button; the page tells the app where that button is when it lays out. The hit test is a pure function from a point and the button's rectangle to the answer, with `cargo test`s. (Built, and the hit test is tested; the flyout itself is a hand check.)
- [ ] Dragging the empty part of the bar moves the window, double-clicking it maximizes and restores, and dragging it to the screen's top edge snaps as usual.
- [ ] The window resizes from all four edges, the top edge included, and keeps Windows 11's rounded corners and shadow.
- [ ] Alt+Space opens the window menu, and its entries work.
- [ ] The bar sits on Mica with the document, and is solid where Mica is off (transparency effects off, Windows 10's path). Check that Mica still shows with the native frame off.
- [ ] The bar's text and glyphs dim when the window isn't active.
- [x] The scroll bar starts below the title bar, not behind it. If that needs the document view to become its own scroll container, check what it changes in `page/src/reading-position.ts` and ADR 0003's rule, and say so in this ticket. (It does; see Comments.)
- [x] High contrast (emulated `forced-colors: active`, and a real contrast theme if one is at hand): the bar uses system colors and stays usable. (Emulated in Chromium; a real contrast theme is in the hand checks.)
- [x] First paint with `npm run measure` stays within the v1 spec's targets, and the window still opens hidden and shows once painted (v1 ticket 04). (On the medians, in two sessions on a busy machine; see Results.)
- [ ] Checked by hand and recorded here: snap layouts, dragging, double-click, the top edge, Alt+Space, maximized on a second monitor, and 100%, 150% and 200% display scaling.

## Results

**2026-11-13, measured** with `npm run measure` on the release build, on the work machine (Windows 11 build 26200), median of 5 runs after one discarded run, in two sessions. The baseline is v1 ticket 07's last measurement, with render ahead.

| Case | Target | Ticket 07 | Session 1 | Session 2 |
| --- | --- | ---: | ---: | ---: |
| Cold start, 100 KB, first paint / shown | under 1.0 s | 760 / 770 ms | **891 / 905 ms** (one run 1075 ms) | **816 / 828 ms** |
| Second window, 100 KB, first paint / shown after the window starts opening | under 300 ms | 172 ms shown | **278 / 291 ms** (two runs 300 and 318 ms shown) | **243 / 258 ms** |
| 1 MB mixed, first paint / shown | under 2 s | 1064 / 1071 ms | 1185 / 1198 ms | 1467 / 1484 ms |
| 1 MB prose, first paint / shown | under 2 s | 691 / 698 ms | 849 / 863 ms | 1137 / 1152 ms |
| 1 MB mixed and prose: longest task or frame once the page has the document | never over 100 ms | none | none | none |
| The same while scrolling through them | never over 100 ms | none | 1 MB mixed: 234 and 100.4 ms in 2 of 5 runs | **none in any run** |
| Read and render (Rust, unchanged): 100 KB / 1 MB mixed / 1 MB prose | | 490 / 881 / 50 ms | 459 / 941 / 46 ms | 481 / 1156 / 50 ms |

- **Every run reported `window-shown`**, so the window still opens hidden and shows once painted.
- **The machine was busier than in ticket 07, and the two sessions differ a lot.** Work this ticket doesn't touch got slower between them: the core's read and render of the 1 MB mixed document went from 941 to 1156 ms, and a reload's file read from 75 to 344 ms. First paint is 60 to 450 ms above ticket 07's, mostly before the page's script starts (100 KB: 797 and 757 ms against 707), where this ticket adds only 3 KB of script and 2 KB of styles. So the noise hides whatever the change costs, and a measurement on an idle machine would settle it.
- **Scrolling:** the two long frames in session 1 didn't come back in session 2, so they are counted as noise. Scrolling runs in the document's own scroller now.
- Find's typing had tasks over 100 ms in some runs of both sessions (up to 154 ms and 183 ms). Not looked into yet: compare with v1 ticket 14's numbers on an idle machine.

## Comments

**2026-11-13, from the grilling.** Tauri's guide for a custom title bar (`data-tauri-drag-region`, `getCurrentWindow().minimize()` and so on) covers dragging and the buttons but not snap layouts. That part is Win32 in `app/src/platform/windows.rs`, the only place `unsafe` lives. If v1 ticket 28 has landed, the window options (decorations, the hit-test hook) go in its one place for what a new window starts with.

**2026-11-13, built; the hand checks are left.** What was built, and what it decided:

- **Snap layouts.** WebView2's windows belong to another process, so Windows never asks Downlight's window what is under the pointer while it is over the page. A small child window of the window's own thread (`DOWNLIGHT_MAXIMIZE_BUTTON`, `app/src/platform/windows/title_bar.rs`) sits over the page's maximize button and answers `HTMAXBUTTON`, as Windows Terminal's drag bar does; Tauri covers the top resize border the same way (`TAURI_DRAG_RESIZE_BORDERS`). The page reports the button's rectangle in device pixels from a `ResizeObserver` on the bar (`device-pixel-content-box`, so window resizes, DPI changes and zoom all report). The pointer over that window never reaches the page, so the window does what the page's button would: it sends the page `maximize-button` (`normal`, `hovered`, `pressed`) for the button's look, and maximizes or restores on a click (`SC_MAXIMIZE`/`SC_RESTORE`). The page's own click (`toggle_maximize_window`) is what other systems use, and Windows before the page has placed the button. `hit_test` is the pure function, with `cargo test`s: on the button it answers `HTMAXBUTTON`; beside it, and in the top resize border of a window that isn't maximized, `HTTRANSPARENT`, so the resize border (Tauri's, on the same thread) answers there; a maximized window's button reaches the screen's top edge. If the window can't be created, only the flyout is missing.
- **Dragging uses WebView2's `app-region: drag`, not Tauri's `data-tauri-drag-region`**, which the spec named. wry already turns on WebView2's non-client region support (`IsNonClientRegionSupportEnabled`), and with it WebView2 drags the window, maximizes and restores it on a double-click, snaps at the top edge and shows the window menu on a right-click, natively and with no IPC or capability. tao keeps `WS_CAPTION`, `WS_SYSMENU` and `WS_MAXIMIZEBOX` on a frameless window and removes only the frame (`WM_NCCALCSIZE`), so Alt+Space, Aero snap and Windows 11's rounded corners and shadow (Tauri's `shadow`, on by default) stay.
- **Only Windows drops its frame.** `platform::draws_own_title_bar()` is true on Windows and false elsewhere, where the native frame stays until the ports decide (ADR 0005). The app tells the page with a `titleBar` flag in its URL, beside `timing` and `backdrop`; without it the page draws no bar. The page reads the flags in one place (`pageFlags` in `page/src/app.ts`), checked against a contract sample of `page_url`'s output (`app/tests/fixtures/page_url.json`), so a flag renamed on one side fails a test. The bar's buttons are commands in `page/src/commands.ts` (`commandButton`), so the close button and Ctrl+W run the same command and a tooltip names the shortcut ("Close (Ctrl+W)"). Setting it to false on Windows brings the native title bar back with nothing else to undo.
- **Maximize or restore** comes from the app: on every resize it sends `window-state` (`maximized` or `restored`). A window opens restored, so the page starts with maximize; when ticket 16 restores a maximized window, the app has to send the state once the page listens.
- **Dimming** follows the page's `focus` and `blur` on the window: wry hands WebView2 the focus whenever the window is activated.
- **The scroll bar below the bar needed the document to scroll in its own element.** The root's scroll bar always spans the whole window, so `view.ts` puts the "File deleted or moved" bar, the document view and the empty state into `.view-scroller`, fixed below the bar. In `reading-position.ts`, the view's top is now the scroller's top plus the scroller's `scroll-padding-top` (which moved there from `:root`), and `scrollToMiddle` measures against and scrolls the scroller; `scrollIntoView` needs no change. Reading-position never looks the scroller up: `view.ts` makes it, owns its selector (`SCROLLER_SELECTOR`), and hands it over with the view (`ScrolledView`, which the `whenDocumentShown` listeners get too, so Find passes it on). The tests' `layOut` places the scroller below a 32 px title bar, as in the window, so a reading position or a Find scroll that forgets the scroller's top fails a test. **ADR 0003's rule is unchanged**: the blocks keep the article as their parent, so `article.markdown-body > *` still applies. Two things the root scroller did for free are now the page's: the scroll keys scroll only the focused element's scroller, so the scroller takes the focus at the start and whenever the focus falls back to `<body>` (Find's Esc), and the bar's buttons never take the focus (`tabindex="-1"`, `mousedown` prevented), as Windows' own don't. The tests that spied on `window.scrollTo`/`scrollBy` spy on the scroller now, and `npm run measure` scrolls it.
- **Icons:** the 16 px "regular" glyphs (`subtract`, `square`, `square_multiple`, `dismiss`); `square_multiple` has no 12 px size. `@fluentui/svg-icons` ships no license file, so `licenses.md` lists it as MIT without the text; its notice goes at the top of the page's bundle from `vite.config.ts` (`postBanner`), where minifying keeps it.
- **Size:** `downlight.exe` grew by 22 KB (7,719,936 to 7,741,952 bytes); the page's script from 35.6 to 38.9 KB, 1.2 KB of it the icons' notice, and its stylesheet from 33.1 to 34.9 KB.
- **Not done here:** the maximize button has no tooltip on Windows, since the pointer never reaches the page there (the flyout takes its place on Windows 11). On Windows 10, tao draws a frameless window with a shadow with a 1 px light border.

**Hand checks** (on the release build, `npm run tauri build`), to record here:

1. Hovering maximize shows the snap layouts flyout, and a zone snaps the window. Maximize looks hovered and pressed, a click maximizes, and the glyph turns into restore.
2. Dragging the empty bar moves the window; a double-click maximizes and restores; dragging to the screen's top edge snaps; a right-click opens the window menu.
3. The window resizes from all four edges and the corners, the top edge too (also its top few pixels above maximize while restored), and keeps rounded corners and a shadow.
4. Alt+Space opens the window menu, and Restore, Move, Size, Minimize, Maximize and Close work.
5. Mica shows behind the bar and the document with the frame off; with "Transparency effects" off the bar is solid; Windows 10 if one is at hand.
6. The glyphs dim when another window is active, and come back on activation.
7. Page Down, Space and the arrows scroll right after opening, after Ctrl+F then Esc, and after a click on minimize and restoring.
8. A real contrast theme: the bar stays usable.
9. Maximized on a second monitor, and at 100 %, 150 % and 200 % scaling: the flyout comes up over the maximize button, also after moving the window between monitors of different scaling.
