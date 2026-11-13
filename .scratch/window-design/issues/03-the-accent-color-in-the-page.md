# 03: The accent color in the page

**What to build:** Links, the selection, Find's matches, ticks, focus rings and the copy button take the accent color, in the shades the spec gives for light and dark. GitHub's own colors stay for everything else.

Spec: [Window design](../spec.md)

**Blocked by:** 02 (The amber icon and the accent scale)

**Status:** ready-for-agent

- [ ] Link text: Dark 2 in light, Light 1 in dark.
- [ ] The selection's background: Light 2 in light, Dark 2 in dark, with the text still readable.
- [ ] Find: every match Light 3 in light and Dark 3 in dark; the current match Base in both, with dark text.
- [ ] Ticked checkboxes, focus rings (today `--borderColor-accent-emphasis`) and the copy button's hover: Dark 1 in light, Light 1 in dark.
- [ ] Each use refers to a shade by its variable, so swapping the scale (ticket 09) recolors them all.
- [ ] Alerts, code highlighting, tables and headings keep GitHub's colors.
- [ ] High contrast keeps its system colors (`Mark`, `Highlight`, `LinkText`), as Find's highlights already do.
- [ ] Checked by hand with `core/tests/fixtures/github.md` in light and dark, on Mica and solid.

## Comments

**2026-11-13, from the grilling.** Base (2.0:1) and Dark 1 (3.3:1) fail 4.5:1 as text on white, so they never color text on a light background. Find's highlights are today `::highlight(find-match)` and `::highlight(find-current)` in `page/src/document.css`.
