# 02: The amber icon and the accent scale

**What to build:** Downlight's amber icon replaces the blue placeholder everywhere, and the amber scale becomes the page's accent color variables. The icon's colors and the scale can't drift apart.

Spec: [Window design](../spec.md)

**Blocked by:** none

**Status:** ready-for-human

- [x] `app/icons/icon.svg` is the amber icon (from `temp/downlight-1h-amber.svg`) without its embedded C2PA metadata, and `icon.ico` is generated from it with `npx tauri icon`, so the exe, the taskbar, Alt+Tab and the installer show it.
- [x] The accent scale is one set of CSS variables in the page, seven shades from Light 3 to Dark 3, with amber's values from the spec.
- [x] The icon's off-white `#FEF9F4` either becomes a shade the scale names or is replaced by Light 3. Choose by looking at the icon at 16 and 32 px, and record the choice here. (Replaced by Light 3; see Comments.)
- [x] A page test reads `app/icons/icon.svg` and the scale and fails when the icon uses a color that isn't one of the scale's shades.
- [ ] Checked by hand: the icon at 16, 24, 32 and 48 px on a light and a dark taskbar, and in Alt+Tab.

## Comments

**2026-11-16, built; the hand check is left.**

- **`#FEF9F4` is replaced by Light 3 (`#FFEFDC`).** The page's fill was a gradient from `#FEF9F4` at the top to Light 3 at the bottom; it is now a flat Light 3, and the gradient is gone. Both versions were rendered with `npx tauri icon --png 16,24,32,48` and compared: at 16 and 32 px they can't be told apart, since the gradient spans only a few pixels of nearly white. Keeping `#FEF9F4` would have meant an eighth shade that only amber has, while the spec swaps Windows' seven accent shades in for amber's one for one (story 26, ticket 09). If anything, the flat Light 3 helps a little: `#FEF9F4` is almost white (1.05:1 on white), so the page's top edge faded into a light taskbar, and Light 3 (1.1:1) holds it slightly better. Neither is strong against white; the amber shape at the top, the folded corner and the Dark 1 lines carry it there.
- **The icon:** `app/icons/icon.svg` is the source without the C2PA manifest (the `<metadata>` element and the `c2pa` namespace) and with a comment pointing at the scale and its test. Its colors are now Light 3, Light 1, Base and Dark 1.
- **`icon.ico`** came from `npx tauri icon app/icons/icon.svg -o <a folder under target/>`, and only `icon.ico` was copied into `app/icons/`. The command also writes macOS, iOS, Android, Microsoft Store and PNG icons, but the repo tracks only `icon.svg` and `icon.ico`, and `bundle.icon` in `tauri.conf.json` names only `icon.ico`, so the rest wasn't kept. Running it without `-o` would write them all into `app/icons/`.
- **The scale** is `page/src/accent.css`, imported by `main.ts` ahead of `document.css`: `--accent-light-3`, `--accent-light-2`, `--accent-light-1`, `--accent-base`, `--accent-dark-1`, `--accent-dark-2`, `--accent-dark-3`, with amber's values. Nothing uses them yet; ticket 03 wires them into links, the selection, Find and the rest. Its own file because ticket 09 swaps exactly this set for Windows' shades, and the test reads exactly this set.
- **The test** is `page/src/accent.test.ts`: the scale names the seven shades in order; the icon paints only with the scale's shades (every `fill`, `stroke`, `stop-color` and the like, skipping `none` and `url(#...)`); and the icon has no `style` attribute or `<style>` element, where a color would slip past the check. It failed on `#fef9f4` before the icon was changed. Vitest empties every stylesheet it imports by default, `?raw` ones too, so `vite.config.ts` sets `test.css.include` to the one `accent.css?raw` import.

**Hand checks** (on the release build, `npm run tauri build`), to record here:

1. `target/release/downlight.exe` in Explorer shows the amber icon (small, medium and large icons). If it still shows the blue one, Windows' icon cache may be stale; if the exe itself still carries it, `cargo clean -p downlight --release` and build again.
2. A window's taskbar button at 100 %, 150 % and 200 % scaling (16 to 48 px), on a light and a dark taskbar: the page, the amber shape at the top and the two lines read, and the page's edge doesn't vanish on the light taskbar.
3. Alt+Tab shows the amber icon.

**2026-11-16, review fixes.** The icon's test now counts SVG's default black too (a filled shape with no `fill` on itself or an ancestor, a stop without its own `stop-color`), and failed on an unfilled `<rect/>` and on an uncolored `<stop/>`, each added to the icon for the check and removed; the scale's test compares the shade names without their order; the icon's comment holds the exact regeneration recipe; its gradient is `downlight-logo-beam` instead of `gB2`, a rename that leaves the pixels and so `icon.ico` unchanged. Suggestions for the role variables and GitHub's accent variables went to ticket 03, and for applying Windows' shades and their names' one source to ticket 09.
