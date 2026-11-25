# Window design

Status: ready-for-agent

## Problem Statement

Downlight works, but it looks like a placeholder: a blue stand-in icon, Windows' plain title bar with "README.md - docs - Downlight" in it, and every command hidden behind the keyboard or the right-click menu. WinDown, the viewer Downlight replaces, looks like a Windows 11 app: its logo and the file name in a slim title bar, the commands I use most as small buttons beside minimize, maximize and close, and a settings flyout for theme, accent color and backdrop. I want Downlight to look and feel like that, in its own amber colors, without losing what the native title bar gives (snap layouts above all).

And until I have closed a window once, a new window opens at 800 x 600, wider than it is tall: for a page of prose that reads down, it shows little of the document and more width than a line needs.

## Solution

The page draws the window's whole **title bar** (ADR 0005): the amber logo, the document's file name, the zoom level when it isn't 100%, and buttons for Open and Edit, Full width and Keep on top, and Settings, set apart by thin separators, followed by minimize, maximize and close. The Windows module brings back what the native title bar did, snap layouts included. A new amber icon replaces the placeholder everywhere. An amber **accent color** marks what Downlight adds to a document: links, the selection, Find's matches, ticks and switched-on buttons. A settings flyout under the gear lets the reader choose the theme, the **backdrop**, the accent color (amber or Windows' own) and the editor.

Before any window has closed, a new window opens tall and narrow, shaped for reading: 900 px wide and 90% of its screen's work area high, centred on the main screen when no window is open (ticket 17). Once a window has closed, new windows open at its size, as v1 ticket 16 made them.

This reverses parts of the [v1 spec](../downlight-v1/spec.md): story 71's native title bar, "no settings window and no toolbar", "a theme override and an accent color" under Out, and full width and Keep on top under v1.x. It ships in v1, before the release check (v1 ticket 18).

## User Stories

### Title bar

1. As a reader, I want Downlight's logo at the left of the title bar, with "Downlight" when I hover it, so that I know which app this is without its name taking space.
2. As a reader, I want a click on the logo to open the window menu (Restore, Move, Size, Minimize, Maximize, Close), as on every Windows title bar.
3. As a reader, I want the title bar to show only the document's file name, so that it stays calm and short.
4. As a reader, I want to see the document's full path when I hover its file name, so that I can tell which README this is.
5. As a reader, I want a click on the file name to show the file selected in Explorer, so that I can get to its folder in one step.
6. As a reader, I want the zoom level shown next to the file name when it isn't 100%, and a click on it to reset it, so that I notice a zoomed window and can undo it.
7. As a reader, I want Alt+Tab and the taskbar to keep showing "README.md - docs - Downlight", so that I can tell ten READMEs apart there.
8. As a reader, I want the snap layouts flyout when I hover maximize, so that the window behaves like every other window.
9. As a reader, I want to drag the window by its title bar, double-click it to maximize, resize it from the top edge and open the window menu with Alt+Space, as with a native title bar.
10. As a reader, I want the title bar to dim when the window isn't active, so that I can see which window has the focus.
11. As a reader, I want the title bar on Mica with the document, and solid where Mica is off (Windows 10, transparency effects off, high contrast), so that it is always legible.

### Title bar buttons

12. As a reader, I want Open and Edit buttons in the title bar, so that the two commands I use most are one click away.
13. As a reader, I want a **Full width** button (Ctrl+Alt+W) that lets the document fill the window instead of GitHub's reading width, remembered for every window, so that wide tables fit when I want them to.
14. As a reader, I want a **Keep on top** button (Ctrl+Alt+T) that keeps this window above all others until I close it, so that I can read a document beside the program I work in.
15. As a reader, I want switched-on buttons shown in the accent color, so that I can see at a glance that Full width or Keep on top is on.
16. As a reader, I want thin separators between groups of buttons (Open and Edit; Full width, Keep on top and Settings; the window's own buttons), so that the bar reads in groups.
17. As a reader, I want each button's tooltip to name its command and shortcut, so that I learn the keyboard.
18. As a reader, I want Full width, Keep on top and Settings in the right-click menu too, so that every command is reachable from it and the keyboard.

### Accent color and icon

19. As a reader, I want links, the selection, Find's matches, ticks, focus rings and switched-on buttons in the accent color, so that Downlight has one consistent look.
20. As a reader, I want link text dark enough to read on a light background and light enough on a dark one, so that the accent never costs legibility.
21. As a Windows user, I want Downlight's amber icon on the exe, the taskbar, Alt+Tab, the installer and Markdown files, and as the logo in the title bar.
22. As the maintainer, I want the icon's colors to be the accent scale's own, checked by a test, so that changing a shade can't leave the icon behind.

### Settings

23. As a reader, I want a Settings button (Ctrl+,) that opens a flyout under it, so that I can change how Downlight looks while I see the document.
24. As a reader, I want to choose the theme: System (default), Light or Dark.
25. As a reader, I want to choose the backdrop: Mica (default), Mica Alt, Acrylic or None.
26. As a reader, I want to choose the accent color: Amber (default) or Windows' accent color, following Windows when it changes.
27. As a reader, I want to choose the editor there ("Edit with"), so that "Choose editor..." doesn't need its own menu entry.
28. As a reader, I want a change to apply at once to every open window and be remembered, so that I set it once.
29. As a reader, I want Esc, a click outside or the gear again to close the flyout, and Tab to move through it, so that it works without the mouse.

### Counts

30. As a reader, I want the document's lines and words in the file name's tooltip, and my selection's words and characters in a small bubble while I select, so that I can see how long a document or a passage is without a status bar taking space. (Ticket 10.)

### Default window size

31. As a reader opening Downlight for the first time, I want a window taller than it is wide, so that I see more of a document's prose at once.
32. As a reader, I want the first window's text column about 100 characters wide, so that prose reads comfortably and most code blocks and tables still fit without scrolling sideways.
33. As a reader who wants GitHub's full reading width, I want to get it by widening the window or with **Full width**, so that the narrower default costs me nothing.
34. As a reader, I want a margin above and below the first window, so that it doesn't look maximized and its edges stay easy to grab.
35. As a reader on a narrow screen (a portrait screen at a high scale, a small tablet), I want a margin at the sides too, so that the window doesn't touch both edges of the screen.
36. As a reader on a short laptop screen, I want the window to fit the screen, even where that makes it wider than tall, so that no part of it is off the screen.
37. As a reader with a portrait or very tall screen, I want the window as tall as the screen allows, with no fixed cap, so that the screen I chose for reading is used.
38. As a reader who launches Downlight many times a day, I want the first window centred on the main screen each time, so that its bottom edge never reaches under the taskbar.
39. As a reader with two monitors, I want a second window, opened beside the first before any has closed, sized for the screen it opens on, so that a window on a short screen still has its margin.
40. As a reader who sized a window once, I want every new window to open at that size after it closed, so that the default never overrides my choice.
41. As a reader with a kept size, I want new windows placed as before (beside the active window, or where Windows places them), so that the new default changes nothing for me.
42. As a reader, I want the same default whether the window opens from the file manager, the command line, Open, a drop or Paste, and for the empty window, so that every way in looks the same.
43. As a reader who opens a file from the file manager, I want **First paint** no slower with the taller window, so that Downlight stays fast to start.

## Implementation Decisions

### Title bar

- The window's native frame is off (`decorations: false`), and the page draws the whole title bar (ADR 0005). Tauri 2 has no way to place buttons in the native title bar on Windows.
- The Windows module brings back what the native title bar did:
  - the snap layouts flyout, by answering Windows' hit test with `HTMAXBUTTON` over the page's maximize button;
  - the resize border at the top edge;
  - the window menu, from Alt+Space and a click on the logo.
- Dragging, double-click to maximize and the window menu on a right-click use WebView2's own drag region (`app-region: drag`), which wry turns on; ticket 01 says why it replaced Tauri's. Its buttons and the file name are not part of it.
- The window's OS title stays "README.md - docs - Downlight" (v1 story 10). The bar shows only the file name, with the full path as its tooltip. A click on the name reveals the file through the app's existing `reveal_file`. A window that shows no document has "Downlight" in the file name's place, as plain text the window is dragged by (ticket 14).
- The logo is the icon's SVG inlined into the page. The content security policy's `img-src` doesn't allow `'self'`, and an inline SVG needs no change to it.
- **Zoom applies to the document only.** With a page-drawn title bar, the webview's zoom would scale the bar and its buttons too, so v1 ticket 16 zooms the document view (CSS `zoom` on the view) and not the webview. The find bar, notices and title bar keep their size.
- The scroll bar starts below the title bar, not behind it.
- Button and caption icons are inline SVGs from Microsoft's Fluent UI System Icons (MIT, kept with its notice), so they look like Windows 11's on Windows 10 and the later ports too.
- High contrast: the bar uses the system colors (`Canvas`, `CanvasText`, `ButtonText`, `Highlight`).

### Buttons and commands

| Group | Button | Shortcut | Remembered |
| --- | --- | --- | --- |
| 1 | Open | Ctrl+O | |
| 1 | Edit | Ctrl+E | |
| 2 | Full width | Ctrl+Alt+W | for every window, in the settings |
| 2 | Keep on top | Ctrl+Alt+T | no; this window only |
| 2 | Settings | Ctrl+, | |
| 3 | Minimize, Maximize, Close | | |

- Every button is a command in the page's command table (`page/src/commands.ts`), so the button, the shortcut and the right-click menu entry run the same command.
- The right-click menu stays native. It gains Full width, Keep on top and Settings, with Settings last, and loses "Choose editor...".

### Accent color

- The accent scale has seven shades: Light 3, Light 2, Light 1, Base, Dark 1, Dark 2, Dark 3. Amber's are:

  | Shade | Amber | Contrast on white / on dark (as the scale gives it) |
  | --- | --- | --- |
  | Light 3 | `#FFEFDC` | 1.1 / 14.5 |
  | Light 2 | `#FFDBAF` | 1.3 / 12.4 |
  | Light 1 | `#FEC173` | 1.6 / 10.2 |
  | Base | `#FBA613` | 2.0 / 8.2 |
  | Dark 1 | `#C27F05` | 3.3 / 4.9 |
  | Dark 2 | `#8F5D04` | 5.6 / 2.9 |
  | Dark 3 | `#623F06` | 9.4 / 1.7 |

- Windows' accent color comes in the same seven shades (`UISettings.GetColorValue`, AccentLight3 to AccentDark3), so choosing it swaps the scale and nothing else.
- The scale is defined once, as CSS variables in the page. Each use takes a shade:

  | Use | Light | Dark |
  | --- | --- | --- |
  | Link text | Dark 2 | Light 1 |
  | Selection background | Light 2 | Dark 2 |
  | Find, every match | Light 3 | Dark 3 |
  | Find, the current match | Base | Base |
  | Ticked checkbox, focus ring, switched-on button, the copy button's hover | Dark 1 | Light 1 |

  Base and Dark 1 fail 4.5:1 as text on white, so they never color text on a light background.
- Alerts, code highlighting, tables and headings keep GitHub's colors. Amber links are the one place documents look less like GitHub (v1 story 14), by choice.
- The icon uses only the scale's colors, and a test fails when it doesn't. The logo stays amber when the accent color is Windows'.

### Settings

- The settings flyout is drawn by the page and anchored under the gear. It is not a window of its own and doesn't cover the document, so each choice shows on the document at once.
- Its controls: Theme, Backdrop, Accent color, and Edit with (the `.txt` program as "Default (name)", the chosen editor, and "Choose..." which opens the file dialog of v1 ticket 15).
- The values go into the settings file that v1 ticket 15 creates. A change applies to every open window at once.
- Theme sets the window's theme in the app, so Mica's tint, the title bar and the page's `prefers-color-scheme` follow it together.
- Backdrop: Mica, Mica Alt (Tauri's `Tabbed`), Acrylic, or None (the solid background used where Mica is off). On Windows 10 the choice is disabled and the background is solid.

### Counts

- No status bar: it would take a row from the document and the corner where the link target bubble sits. The document's counts go into the file name's tooltip, under the path ("1,234 lines, 5,678 words"); the selection's show in a bubble at the bottom right while text in the document is selected ("12 words, 68 characters").
- Lines are the file's, as line numbers are; words and characters are of the text the reader sees, split by UAX #29. Not counted: the document's characters, a reading time, the reading position's line, the encoding.
- The core counts the document while it renders, so the counts cost the page nothing and a 100 MB document holds nothing up; the page counts only the selection (`Intl.Segmenter`). Ticket 10 has the details.

### Default window size

- **Only before any window has closed.** The size kept in the settings (the last window that closed, v1 ticket 16) still wins over the default, and the smallest size (320 x 240) and the cascade down and to the right of the active window stay as they are.
- **The default comes from a work area:** width 900 logical px, at most 90% of the work area's width; height 90% of the work area's height, at least 600 px, with no upper limit. The result is then fitted to the work area as every new window is, so the 600 px floor never puts a window past a short screen's edge.
- **Whose work area:** the screen the window opens on. With an active window at a place and size of its own, that is the active window's screen, which the placement already knows from the active window's place; without one, the main screen's. An active window that is minimized, maximized or in full screen has no place of its own (the `Windows` port reports none for it, v1 ticket 16) and counts as none: the window is shaped for the main screen and centred on it, as with no active window. The placement already takes both, so its inputs don't change: the kept size, the active window's place and the main screen's work area.
- **Placed by Downlight without an active window.** A window at the default size with no active window gets a place: centred in the main screen's work area, in that screen's logical pixels, which the window opens at. Left to the system (no position, which tao turns into `CW_USEDEFAULT`), Windows moves each new window down by about a title bar, and fitting only shrinks a window, so a window 90% of the screen high would reach under the taskbar after a few launches. A window at a kept size without an active window is still placed by the system, as today, so a reader's own habit is left alone.
- **No new port and no change outside the placement:** the `Windows` port already reports the active window's place with its work area and the main screen's work area, and a new window already carries an optional place.
- Rejected: a fixed portrait size (900 x 1100), which doesn't fit most laptops; about 1000 px wide (GitHub's reading width with the scroll bar), which gives lines of about 115 characters; a cap on the height, which stunts a portrait screen's window; keeping the system's place and moving the window up once it is made, which needs its place afterwards and could flicker; a size per document, which is a different feature.
- How the default comes out, with Windows 11's 48 px taskbar:

  | Screen | Scale | Work area (logical) | Default window |
  | --- | --- | --- | --- |
  | 5120 x 2160 (40" ultrawide) | 100% / 125% / 150% | 5120 x 2112 / 4096 x 1680 / 3413 x 1392 | 900 x 1901 / 900 x 1512 / 900 x 1253 |
  | 1440 x 2560 portrait | 100% | 1440 x 2512 | 900 x 2261 |
  | 3840 x 2160 | 150% / 200% | 2560 x 1392 / 1920 x 1032 | 900 x 1253 / 900 x 929 |
  | 2560 x 1440 | 100% / 125% | 2560 x 1392 / 2048 x 1104 | 900 x 1253 / 900 x 994 |
  | 1920 x 1080 | 100% / 125% / 150% | 1920 x 1032 / 1536 x 816 / 1280 x 672 | 900 x 929 / 900 x 734 / 900 x 605 |
  | 1366 x 768 | 125% | 1093 x 566 | 900 x 566 |
  | 1080 x 1920 portrait | 150% | 720 x 1232 | 648 x 1109 |

  Ticket 17 has the longer table.

### Modules

- **Page:** a title bar module (logo, file name, zoom level, buttons, window buttons), a settings flyout module, and the accent scale as CSS variables. They talk to the app only through `page/src/app.ts`, as every page module does.
- **App:** the settings store gains theme, backdrop, accent color and full width. Keep on top is window state, not a setting.
- **Windows module:** the hit test for snap layouts and the top border, the window menu, Windows' accent shades and their change event, and the backdrop. `unsafe` stays there.

## Testing Decisions

- **Page** (Vitest with happy-dom): the title bar shows the file name, its tooltip is the full path, a click asks the app to reveal it; the zoom level shows only off 100% and a click resets it; each button runs its command, and Full width and Keep on top show their state; the flyout opens, closes on Esc, a click outside and the gear, keeps the focus inside, and sends each change to the app.
- **Contract samples:** each new command and message gets its sample in `app/tests/fixtures/`, as every other does.
- **App** (`cargo test`): the settings store keeps theme, backdrop, accent color and full width across a restart, and loads defaults when they're missing. The hit test is a pure function from a point and the maximize button's rectangle to the hit test answer, and is tested as one.
- **Counts:** the core's tests count lines and words through `render`; the page's tests check the tooltip and the selection bubble; the rendered document's new shape gets its contract sample.
- **Accent scale:** a page test reads the icon's SVG and fails on any color that isn't a shade of the scale.
- **Default window size** (`cargo test`, beside v1 ticket 16's placement tests): through the documents module's public way of opening a window, with the recording fake of the `Windows` port giving the work areas, check the size and place each new window gets: 900 wide and 90% of a work area high with no kept size; 90% of the width on a work area narrower than 1000 px; at least 600 px on one shorter than 667 px, fitted to it; the size from the active window's screen when it is on another monitor; centred on the main screen's work area without an active window; a kept size placed by the system and winning over the default. The tests check only what a window is opened with, never how the placement works it out.
- **By hand**, per ticket: light, dark and high contrast; Windows 10's solid path; snap layouts, dragging, double-click, the top border and Alt+Space; the icon at 16, 24, 32 and 48 px.

## Out of Scope

- A page-drawn right-click menu. It stays native.
- A settings window, and settings beyond the four above.
- A logo that follows the accent color.
- New empty windows from the logo.
- A status bar: the counts (ticket 10) take its place.
- A size or place remembered per document, and a default size the reader sets in the settings: a window's own size, kept when it closes, already does that.
- Changing where a window with a kept size opens without an active window, and the cascade beside the active window.

## Further Notes

- Reference: WinDown's title bar and settings flyout (screenshots of 2026-11-13). Downlight takes the layout, not WinDown's code.
- The amber scale and the icon came with that conversation (`downlight-1h-amber.svg`). The icon's embedded C2PA metadata is removed when it is committed.
- New glossary terms: Title bar, Accent color, Backdrop, Theme, Full width, Keep on top, Settings, and Counts (from ticket 10's grilling).
- The default window size was asked for by the repo owner on 2026-11-23 and grilled on 2026-11-25, checked against the owner's own monitors (a 40" 5120 x 2160 ultrawide at 125%, a 1440 x 2560 portrait screen). It adds no glossary term: **Active window** already says a new window opens at the size of the last window that closed. Ticket 17 is its one ticket, so it needs no `/to-tickets`; its hand checks add a **First paint** check with no kept size, which `npm run measure` doesn't catch because it uses the kept size.
