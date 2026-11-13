# 02: The amber icon and the accent scale

**What to build:** Downlight's amber icon replaces the blue placeholder everywhere, and the amber scale becomes the page's accent color variables. The icon's colors and the scale can't drift apart.

Spec: [Window design](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] `app/icons/icon.svg` is the amber icon (from `temp/downlight-1h-amber.svg`) without its embedded C2PA metadata, and `icon.ico` is generated from it with `npx tauri icon`, so the exe, the taskbar, Alt+Tab and the installer show it.
- [ ] The accent scale is one set of CSS variables in the page, seven shades from Light 3 to Dark 3, with amber's values from the spec.
- [ ] The icon's off-white `#FEF9F4` either becomes a shade the scale names or is replaced by Light 3. Choose by looking at the icon at 16 and 32 px, and record the choice here.
- [ ] A page test reads `app/icons/icon.svg` and the scale and fails when the icon uses a color that isn't one of the scale's shades.
- [ ] Checked by hand: the icon at 16, 24, 32 and 48 px on a light and a dark taskbar, and in Alt+Tab.
