# 04: The logo and the file name in the title bar

**What to build:** The title bar's left side: the amber logo, which shows "Downlight" on hover and opens the window menu on a click; the document's file name, which shows its full path on hover and reveals the file in Explorer on a click; and the zoom level when it isn't 100%, which resets on a click.

Spec: [Window design](../spec.md)

**Blocked by:** 01 (Spike: the page draws the title bar), 02 (The amber icon and the accent scale), v1 [16](../../downlight-v1/issues/16-zoom-and-window-size.md) (Zoom and window size)

**Status:** ready-for-agent

- [ ] The logo is the icon's SVG inlined into the page (the content security policy has no `'self'` in `img-src`). Its tooltip is "Downlight", and a click opens the window menu, as Alt+Space does.
- [ ] The file name shows the shown document's file name and follows it on links, Back, Forward, Open and drops. The empty window shows only the logo.
- [ ] Hovering the name shows the full path. A click reveals the file in Explorer through the app's `reveal_file`; a missing file shows a notice instead.
- [ ] The zoom level (`125%`) shows next to the name only when it isn't 100%, and a click resets it.
- [ ] A long file name is cut with an ellipsis before it pushes the buttons.
- [ ] The window's OS title stays "README.md - docs - Downlight" (v1 story 10).
- [ ] The logo, name and zoom level are not part of the drag region; the rest of the left side is.
- [ ] Page tests: the name and its tooltip after a show and after a link; the reveal request on a click; the zoom level hidden at 100%, shown otherwise, and reset on a click.

## Comments

**2026-11-13, from the grilling.** The reveal is a single click, not a double-click: a double-click on a title bar maximizes on Windows.
