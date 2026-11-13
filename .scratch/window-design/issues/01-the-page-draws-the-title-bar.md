# 01: Spike: the page draws the title bar

**What to build:** The window's native frame goes, and the page draws a plain title bar: an empty drag area and minimize, maximize and close. Everything the native title bar did still works, the snap layouts flyout above all. This ticket proves ADR 0005 before anything is built on it; if snap layouts can't be brought back, stop here and record why.

Spec: [Window design](../spec.md), ADR [0005](../../../docs/adr/0005-the-page-draws-the-title-bar.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] Windows open with `decorations: false`. The page draws a title bar of Windows 11's height (32 px) with minimize, maximize (restore when maximized) and close, using Fluent UI System Icons inlined as SVG, with their MIT notice.
- [ ] Hovering maximize shows Windows' snap layouts flyout. The Windows module answers `WM_NCHITTEST` with `HTMAXBUTTON` over the page's maximize button; the page tells the app where that button is when it lays out. The hit test is a pure function from a point and the button's rectangle to the answer, with `cargo test`s.
- [ ] Dragging the empty part of the bar moves the window, double-clicking it maximizes and restores, and dragging it to the screen's top edge snaps as usual.
- [ ] The window resizes from all four edges, the top edge included, and keeps Windows 11's rounded corners and shadow.
- [ ] Alt+Space opens the window menu, and its entries work.
- [ ] The bar sits on Mica with the document, and is solid where Mica is off (transparency effects off, Windows 10's path). Check that Mica still shows with the native frame off.
- [ ] The bar's text and glyphs dim when the window isn't active.
- [ ] The scroll bar starts below the title bar, not behind it. If that needs the document view to become its own scroll container, check what it changes in `page/src/reading-position.ts` and ADR 0003's rule, and say so in this ticket.
- [ ] High contrast (emulated `forced-colors: active`, and a real contrast theme if one is at hand): the bar uses system colors and stays usable.
- [ ] First paint with `npm run measure` stays within the v1 spec's targets, and the window still opens hidden and shows once painted (v1 ticket 04).
- [ ] Checked by hand and recorded here: snap layouts, dragging, double-click, the top edge, Alt+Space, maximized on a second monitor, and 100%, 150% and 200% display scaling.

## Comments

**2026-11-13, from the grilling.** Tauri's guide for a custom title bar (`data-tauri-drag-region`, `getCurrentWindow().minimize()` and so on) covers dragging and the buttons but not snap layouts. That part is Win32 in `app/src/platform/windows.rs`, the only place `unsafe` lives. If v1 ticket 28 has landed, the window options (decorations, the hit-test hook) go in its one place for what a new window starts with.
