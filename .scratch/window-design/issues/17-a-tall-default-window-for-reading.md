# 17: A tall default window for reading

**What to build:** A Downlight window that opens before any window has closed opens tall and narrow, shaped for reading text, instead of Tauri's 800 x 600, which is wider than it is tall: a page of prose reads down, so the height shows more of it, and the width only needs to hold a comfortable line. Once a window has closed, every new window opens at its size (**Active window**), so a reader who sizes a window once keeps that size.

The default:

- **Width 900 logical px, at most 90% of the work area's width.** The text column is then about 820 px at 100% zoom (the view's 32 px padding each side and the scroll bar taken off), about 100 characters of body text: narrower than GitHub's reading width (980 px with padding), which a reader gets back by widening the window or with **Full width**, and wide enough that most code blocks and tables fit without scrolling sideways. The 90% only matters on a work area narrower than 1000 px (a 1080 x 1920 portrait screen at 150%, a 1024 x 768 tablet), where the window keeps a margin at its sides instead of touching both edges.
- **Height 90% of the work area** (the screen without the taskbar), at least 600 px, with no upper limit: as tall as the screen allows with a margin, so the window doesn't look maximized and its edges stay easy to grab. A portrait or very tall screen gets a very tall window (2261 px on a 1440 x 2560 portrait screen at 100%), which is what such a screen is for; a reader who wants it shorter resizes it once and keeps that size.
- **The work area of the screen the window opens on:** the active window's screen when it is at a place and size of its own (a second window opened before any has closed, which opens beside it, maybe on another monitor), else the main screen. Taking the main screen's height for a window on a shorter screen would leave it fitted to that screen's full height, with no margin. An active window that is minimized, maximized or in **Full screen** has no place of its own and counts as none: the new window is then shaped for the main screen and centred on it, as with no window open.
- **On a screen too short for that** (a 1920 x 1080 laptop at 150% has a work area of 1280 x 672 logical px), the window is as tall as fits and may end up wider than tall; that is the screen's shape, not the default's. Where 600 px is more than the work area, the window is fitted to it, as today.
- **Centred on the main screen's work area when no window is open.** A window at the default size without an active window is placed by Downlight, centred in the main screen's work area (so 5% of it above and below), not by the system. The system's default place (`CW_USEDEFAULT`, which tao passes when given no position) moves each new window down by about a title bar, and fitting only shrinks a window, never moves it, so after a few launches a window 90% of the screen high could reach under the taskbar. A window at a size kept in the settings is still placed by the system, as today, and one with an active window still opens down and to the right of it.
- **Windows opened together cascade** (from the review). A launch naming several documents, or a second launch's hand-over, opens its windows before the reader is in any of them, so each new window opens down and to the right of the window opened just before it, until the reader is in a window again; that window is forgotten if it closes first. Without this, every default-sized window of `downlight a.md b.md` would be centred on the same point, where the system used to cascade them. A window the system placed is never one to open beside, since the system cascades its own.

How the default comes out (work areas with Windows 11's 48 px taskbar; this repo owner's own monitors marked):

| Screen | Scale | Work area (logical) | Default window |
| --- | --- | --- | --- |
| 5120 x 2160, 40" ultrawide (owner's main) | 125% | 4096 x 1680 | 900 x 1512 |
| 5120 x 2160 | 100% / 150% | 5120 x 2112 / 3413 x 1392 | 900 x 1901 / 900 x 1253 |
| 1440 x 2560 portrait (owner's second) | 100% | 1440 x 2512 | 900 x 2261 |
| 3440 x 1440 ultrawide | 100% | 3440 x 1392 | 900 x 1253 |
| 3840 x 2160 | 150% / 200% | 2560 x 1392 / 1920 x 1032 | 900 x 1253 / 900 x 929 |
| 2560 x 1440 | 100% / 125% | 2560 x 1392 / 2048 x 1104 | 900 x 1253 / 900 x 994 |
| 2560 x 1600 laptop | 150% | 1707 x 1019 | 900 x 917 |
| 2880 x 1920 Surface | 200% | 1440 x 912 | 900 x 821 |
| 1920 x 1080 | 100% | 1920 x 1032 | 900 x 929 |
| 1920 x 1080 laptop | 125% / 150% | 1536 x 816 / 1280 x 672 | 900 x 734 / 900 x 605 |
| 1920 x 1200 laptop | 150% | 1280 x 752 | 900 x 677 |
| 1366 x 768 | 100% / 125% | 1366 x 720 / 1093 x 566 | 900 x 648 / 900 x 566 |
| 1080 x 1920 portrait | 150% | 720 x 1232 | 648 x 1109 |

Rejected: a fixed portrait size such as 900 x 1100, which doesn't fit most laptops and would be cut to fit anyway; a width of GitHub's full reading width plus the scroll bar (about 1000 px), which gives lines of about 115 characters, longer than is comfortable for prose; an upper limit on the height, which would make a portrait screen's window look stunted; keeping the system's placement and moving the window up afterwards when it runs past the bottom, which needs the window's place after it was made and could flicker; and remembering a size per document, which is a different feature.

Spec: none; a follow-up to window design's "a new window opens at the size of the last window that closed" (**Active window**). Asked for by the repo owner on 2026-11-23, grilled on 2026-11-25.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The default size is no longer a fixed 800 x 600: before any window has closed, a new window is 900 px wide, at most 90% of its screen's work area wide, and 90% of that work area high, at least 600 px, made to fit the screen as today.
- [ ] The work area is the active window's screen's when the active window is at a place and size of its own, else the main screen's. An active window that is minimized, maximized or in full screen counts as none: the new window is shaped for the main screen and centred on it, as with no window open.
- [ ] With no window open, a window at the default size is centred in the main screen's work area; one at a size kept in the settings is still placed by the system.
- [ ] Once a window has closed, new windows open at its size, as today; the default never overrides a size kept in the settings.
- [ ] The smallest size (320 x 240) and the cascade down and to the right of the active window are unchanged.
- [ ] A new window opened from the command line, Open, a drop or Paste with no window open and no size kept gets the new default; so does the empty window.
- [ ] App tests (written first, beside the existing placement tests in `app/src/documents/tests/place.rs`): with no last size, the size is 900 wide and 90% of a given work area high; a work area narrower than 1000 px gives 90% of its width; a work area shorter than 667 px gives at least 600 px, fitted to the screen; a small work area fits the window into it; with an active window on another screen, the default comes from that screen's work area; with no active window the default-sized window is centred on the main screen's work area, and a kept size leaves the place to the system; a last size kept in the settings wins over the default.
- [ ] Hand check in the release build, with the window size removed from the settings file: on the 5120 x 2160 screen at 125% the first window is 900 x 1512 and centred; opened several times in a row (closing it in between after removing the kept size again, or with the settings file read-only), it never reaches under the taskbar; a second window opened while the first is open, with the first on the portrait screen, is 900 x 2261 and opens beside it; on a laptop at 150% the window fits the screen.
- [ ] Hand check of first paint, with no size kept in the settings: `DOWNLIGHT_TIMING`'s first-paint line on the 1 MB document stays within ticket 02's baseline (`npm run measure` uses whatever size the settings keep, so it doesn't catch this).
- [ ] `npm run check` passes.
