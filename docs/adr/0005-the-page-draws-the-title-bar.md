# The page draws the title bar

The window's native frame is off, and the page draws the whole title bar: the logo, the document's file name, Downlight's buttons, and minimize, maximize and close. We chose this because Downlight's most used commands (Open, Edit, Full width, Keep on top, Settings) belong next to the window's own buttons, as in WinDown, and Tauri 2 has no way to put anything into the native title bar on Windows. It reverses v1 story 71 ("the native title bar with Windows' snap layouts"), with one condition kept from it: the snap layouts flyout on maximize must still work. The Windows module answers Windows' hit test with `HTMAXBUTTON` over the page's maximize button to get it back, and the first ticket proves that before anything else is built on it ([window design](../../.scratch/window-design/spec.md), ticket 01).

## Considered options

- **Keep the native title bar** and put the buttons in a toolbar under it. Snap layouts and everything else come for free, but it costs a second strip of height, and the window looks like an old desktop app rather than a Windows 11 one.
- **Extend the page into the native title bar** (`DwmExtendFrameIntoClientArea` with a custom `WM_NCCALCSIZE`) and keep Windows' own caption buttons. Windows would draw minimize, maximize and close and handle snap layouts, but WebView2 is a child window over the whole client area and covers them, and keeping the two in line is more Win32 code, not less.
- **Drop snap layouts.** The page draws the bar with plain buttons, as Tauri's guide does. Least code, but the window then behaves unlike every other window on hover over maximize, which is what story 71 wanted to avoid.

## Consequences

- What the native title bar did is now Downlight's to do: dragging, double-click to maximize, the resize border at the top edge, the window menu (Alt+Space and a click on the logo), dimming when the window isn't active, and high contrast. Each is checked by hand.
- The webview's zoom would scale the title bar too, so zoom applies to the document view only (CSS `zoom`), not to the webview.
- The OS title of the window still carries the file name, its folder and "Downlight" for Alt+Tab and the taskbar, while the bar shows only the file name.
- The macOS and Linux ports have to decide their own title bar: macOS's traffic lights and Linux's window managers differ, and the page's caption buttons are Windows' layout.
