# Window design

Status: ready-for-agent

## Problem Statement

Downlight works, but it looks like a placeholder: a blue stand-in icon, Windows' plain title bar with "README.md - docs - Downlight" in it, and every command hidden behind the keyboard or the right-click menu. WinDown, the viewer Downlight replaces, looks like a Windows 11 app: its logo and the file name in a slim title bar, the commands I use most as small buttons beside minimize, maximize and close, and a settings flyout for theme, accent color and backdrop. I want Downlight to look and feel like that, in its own amber colors, without losing what the native title bar gives (snap layouts above all).

## Solution

The page draws the window's whole **title bar** (ADR 0005): the amber logo, the document's file name, the zoom level when it isn't 100%, and buttons for Open and Edit, Full width and Keep on top, and Settings, set apart by thin separators, followed by minimize, maximize and close. The Windows module brings back what the native title bar did, snap layouts included. A new amber icon replaces the placeholder everywhere. An amber **accent color** marks what Downlight adds to a document: links, the selection, Find's matches, ticks and switched-on buttons. A settings flyout under the gear lets the reader choose the theme, the **backdrop**, the accent color (amber or Windows' own) and the editor.

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

### Later

30. As a reader, I want a small status bar at the bottom with line, word and selection counts. (Needs triage: ticket 10.)

## Implementation Decisions

### Title bar

- The window's native frame is off (`decorations: false`), and the page draws the whole title bar (ADR 0005). Tauri 2 has no way to place buttons in the native title bar on Windows.
- The Windows module brings back what the native title bar did:
  - the snap layouts flyout, by answering Windows' hit test with `HTMAXBUTTON` over the page's maximize button;
  - the resize border at the top edge;
  - the window menu, from Alt+Space and a click on the logo.
- Dragging and double-click to maximize use Tauri's drag region. Its buttons and the file name are not part of it.
- The window's OS title stays "README.md - docs - Downlight" (v1 story 10). The bar shows only the file name, with the full path as its tooltip. A click on the name reveals the file through the app's existing `reveal_file`.
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

### Modules

- **Page:** a title bar module (logo, file name, zoom level, buttons, window buttons), a settings flyout module, and the accent scale as CSS variables. They talk to the app only through `page/src/app.ts`, as every page module does.
- **App:** the settings store gains theme, backdrop, accent color and full width. Keep on top is window state, not a setting.
- **Windows module:** the hit test for snap layouts and the top border, the window menu, Windows' accent shades and their change event, and the backdrop. `unsafe` stays there.

## Testing Decisions

- **Page** (Vitest with happy-dom): the title bar shows the file name, its tooltip is the full path, a click asks the app to reveal it; the zoom level shows only off 100% and a click resets it; each button runs its command, and Full width and Keep on top show their state; the flyout opens, closes on Esc, a click outside and the gear, keeps the focus inside, and sends each change to the app.
- **Contract samples:** each new command and message gets its sample in `app/tests/fixtures/`, as every other does.
- **App** (`cargo test`): the settings store keeps theme, backdrop, accent color and full width across a restart, and loads defaults when they're missing. The hit test is a pure function from a point and the maximize button's rectangle to the hit test answer, and is tested as one.
- **Accent scale:** a page test reads the icon's SVG and fails on any color that isn't a shade of the scale.
- **By hand**, per ticket: light, dark and high contrast; Windows 10's solid path; snap layouts, dragging, double-click, the top border and Alt+Space; the icon at 16, 24, 32 and 48 px.

## Out of Scope

- A page-drawn right-click menu. It stays native.
- A settings window, and settings beyond the four above.
- A logo that follows the accent color.
- New empty windows from the logo.
- The status bar: ticket 10 collects what is known, and it gets its own grilling before it is built.

## Further Notes

- Reference: WinDown's title bar and settings flyout (screenshots of 2026-11-13). Downlight takes the layout, not WinDown's code.
- The amber scale and the icon came with that conversation (`downlight-1h-amber.svg`). The icon's embedded C2PA metadata is removed when it is committed.
- New glossary terms: Title bar, Accent color, Backdrop, Theme, Full width, Keep on top, Settings.
