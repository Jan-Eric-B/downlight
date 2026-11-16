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

**2026-11-16, from ticket 02's review.** Suggestions, not decisions:

- Consider one role variable per row of the spec's use-to-shade table in `page/src/accent.css` (link, selection, Find match, current match, emphasis), each defined per `prefers-color-scheme` from the shades and mapped to system colors under `forced-colors: active`, so `document.css` uses only roles and never picks a shade itself.
- Don't redefine GitHub's `--fgColor-accent` or `--borderColor-accent-emphasis`: GitHub's Note alert uses both (`node_modules/github-markdown-css/github-markdown.css`, lines 1201-1207), and this ticket keeps alerts in GitHub's colors. Recolor at the selectors instead, and replace `var(--borderColor-accent-emphasis)` in the focus rings in `document.css` (lines 217, 279 and 410 today). github-markdown-css's own focus rings use `--focus-outlineColor` (lines 356, 372 and 1129), which it sets from `--borderColor-accent-emphasis`; that one variable could take the role without touching the alert.
