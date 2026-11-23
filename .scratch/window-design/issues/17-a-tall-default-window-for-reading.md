# 17: A tall default window for reading

**What to build:** The first Downlight window, before any window has closed, opens tall and narrow, shaped for reading text, instead of Tauri's 800 x 600, which is wider than it is tall: a page of prose reads down, so the height shows more of it, and the width only needs to hold a comfortable line. Every later window still opens at the size of the last window that closed (**Active window**), so a reader who sizes a window once keeps that size.

The default:

- **Width 900 logical px.** The text column is then about 820 px at 100% zoom (the view's 32 px padding each side and the scroll bar taken off), about 100 characters of body text: narrower than GitHub's reading width (980 px with padding), which a reader gets back by widening the window or with **Full width**, and wide enough that most code blocks and tables fit without scrolling sideways.
- **Height 90% of the screen's work area** (the screen without the taskbar), at least 600 px: as tall as the screen allows with a margin, so the window doesn't look maximized and its edges stay easy to grab.
- **On a screen too short for that** (a 1920 x 1080 laptop at 150% scaling has a work area of about 1280 x 690 logical px), the window is as tall as fits and may end up wider than tall; that is the screen's shape, not the default's.
- **Placed by the system, as today**, on the main screen, and still made to fit it.

Rejected: a fixed portrait size such as 900 x 1100, which doesn't fit most laptops and would be cut to fit anyway; a width of GitHub's full reading width plus the scroll bar (about 1000 px), which gives lines of about 115 characters, longer than is comfortable for prose; and remembering a size per document, which is a different feature.

Spec: none; a follow-up to window design's "a new window opens at the size of the last window that closed" (**Active window**). Asked for by the repo owner on 2026-11-23.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The default size is no longer a fixed 800 x 600: before any window has closed, a new window is 900 px wide and 90% of the main screen's work area high, at least 600 px, made to fit the screen as today.
- [ ] Once a window has closed, new windows open at its size, as today; the default never overrides a size kept in the settings.
- [ ] The smallest size (320 x 240) and the cascade down and to the right of the active window are unchanged.
- [ ] A new window opened from the command line, Open, a drop or Paste with no window open and no size kept gets the new default; so does the empty window.
- [ ] App tests (written first, beside the existing placement tests): with no last size, the size is 900 wide and 90% of a given work area high; a work area shorter than 667 px gives at least 600 px, fitted to the screen; a small work area fits the window into it; a last size kept in the settings wins over the default.
- [ ] Hand check in the release build, with the window size removed from the settings file: on a 1920 x 1080 screen at 100% the first window is taller than wide, and on a laptop at 150% it fits the screen.
- [ ] `npm run check` passes.
