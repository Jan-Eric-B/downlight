# 03: The accent color in the page

**What to build:** Links, the selection, Find's matches, ticks, focus rings and the copy button take the accent color, in the shades the spec gives for light and dark. GitHub's own colors stay for everything else.

Spec: [Window design](../spec.md)

**Blocked by:** 02 (The amber icon and the accent scale)

**Status:** ready-for-human

- [x] Link text: Dark 2 in light, Light 1 in dark.
- [x] The selection's background: Light 2 in light, Dark 2 in dark, with the text still readable.
- [x] Find: every match Light 3 in light and Dark 3 in dark; the current match Base in both, with dark text.
- [x] Ticked checkboxes, focus rings (today `--borderColor-accent-emphasis`) and the copy button's hover: Dark 1 in light, Light 1 in dark.
- [x] Each use refers to a shade by its variable, so swapping the scale (ticket 09) recolors them all.
- [x] Alerts, code highlighting, tables and headings keep GitHub's colors.
- [x] High contrast keeps its system colors (`Mark`, `Highlight`, `LinkText`), as Find's highlights already do.
- [ ] Checked by hand with `core/tests/fixtures/github.md` in light and dark, on Mica and solid.

## Comments

**2026-11-13, from the grilling.** Base (2.0:1) and Dark 1 (3.3:1) fail 4.5:1 as text on white, so they never color text on a light background. Find's highlights are today `::highlight(find-match)` and `::highlight(find-current)` in `page/src/document.css`.

**2026-11-16, from ticket 02's review.** Suggestions, not decisions:

- Consider one role variable per row of the spec's use-to-shade table in `page/src/accent.css` (link, selection, Find match, current match, emphasis), each defined per `prefers-color-scheme` from the shades and mapped to system colors under `forced-colors: active`, so `document.css` uses only roles and never picks a shade itself.
- Don't redefine GitHub's `--fgColor-accent` or `--borderColor-accent-emphasis`: GitHub's Note alert uses both (`node_modules/github-markdown-css/github-markdown.css`, lines 1201-1207), and this ticket keeps alerts in GitHub's colors. Recolor at the selectors instead, and replace `var(--borderColor-accent-emphasis)` in the focus rings in `document.css` (lines 217, 279 and 410 today). github-markdown-css's own focus rings use `--focus-outlineColor` (lines 356, 372 and 1129), which it sets from `--borderColor-accent-emphasis`; that one variable could take the role without touching the alert.

**2026-11-16, built; the hand check is left.**

- **The roles** are five variables in `page/src/accent.css`, one per row of the spec's use-to-shade table: `--accent-link`, `--accent-selection`, `--accent-find-match`, `--accent-find-current` and `--accent-emphasis` (ticked checkbox, focus ring, switched-on button, the copy button's hover). Each is defined from the shades under `prefers-color-scheme: light` and `dark`, and as a system color under `forced-colors: active`: `LinkText`, `Highlight`, `Mark`, `Highlight` and `Highlight`. The forced-colors block comes last, so it wins over light and dark. `document.css` uses only the roles and never a shade, so ticket 09's swap of the scale recolors every use. The switched-on buttons of tickets 06 and 07 take `--accent-emphasis`, and `accent.test.ts` fails if a stylesheet it reads names a shade or an unknown role, so a new use joins a row of the table rather than picking a shade. That test reads only `document.css`; if the title bar's styles move to their own file, add it there.
- **GitHub's accent variables are untouched**, as the review suggested: `document.css` recolors at the selectors (`.markdown-body a`, `::selection`, `.markdown-body input[type="checkbox"]` with `accent-color`, Find's two highlights), and sets github-markdown-css's `--focus-outlineColor` to the emphasis role on `.markdown-body`, which recolors its own focus rings on links and checkboxes. The page's own three focus rings (copy button, Open file, the find box) take the role directly. So the Note alert stays blue, and so does the footnote that is the URL's `:target` (github-markdown-css draws its frame in `--borderColor-accent-emphasis`); that frame isn't a focus ring, and Downlight's heading links scroll without changing the URL, so it rarely shows.
- **The copy button's hover** turns its icon to the emphasis role and keeps GitHub's hover background; "Copied" keeps GitHub's green. In high contrast the icon is `Highlight` under the pointer, `ButtonText` otherwise.
- **The current match's text** stays dark in both themes (`#1f2328` in light, `#0d1117` in dark, as before), about 8:1 and 9.5:1 on Base. **The selection** sets only its background, so text keeps its own color: GitHub's text on Light 2 is about 12:1, a link (Dark 2) on it about 5.7:1; in dark, GitHub's text on Dark 2 is about 7:1, a link (Light 1) on it about 4.6:1. In high contrast the selection's text is `HighlightText`, and Find's are `MarkText` and `HighlightText`, as before.
- **Light 3 for every match** is faint on white (1.1:1), about as faint as Chromium's own find yellow; the hand check should say whether it is enough on Mica too.
- **Ticks:** the core writes task checkboxes `disabled` until v1 ticket 13 makes them tickable, and Chromium may draw a disabled checkbox gray whatever its `accent-color`. If the hand check shows gray ticks, that is why, and ticket 13 brings the amber.
- **The tests** are in `page/src/accent.test.ts`, written first and failing before the roles existed: each role takes the spec's shade in light and in dark; in high contrast each takes its system color; `document.css` uses exactly the roles and no shade; and neither stylesheet defines or uses `--fgColor-accent` or `--borderColor-accent-emphasis`. They read the roles through happy-dom's CSSOM (`CSSMediaRule`, `CSSStyleRule`), and `vite.config.ts` now keeps `document.css?raw` too.

**2026-11-16, review fixes.** The five roles are one `:root` block in `accent.css` now, each `light-dark(<light shade>, <dark shade>)` as `document.css` pairs the themes (the root's `color-scheme` is `light dark`), with the forced-colors block still last; the test checks each role is exactly that pair. `accent.test.ts` reads every stylesheet of the page (`import.meta.glob("./*.css")`, so `title-bar.css`, split from `document.css` in ticket 04's review, and any later one too): together they use exactly the roles and no shade, and none names GitHub's two accent variables; `vite.config.ts` keeps any `page/src/*.css?raw`. A new test checks each use takes its row's role, as a selector, property and role table (`.markdown-body a` color is link, the copy button's hover and every focus ring emphasis, and so on), and fails on a wrong role: it failed with Find's two roles swapped and with the link's and the checkbox's swapped, each swap made for the check and undone. happy-dom drops rules with a `::highlight()` selector, so the test reads Find's two from the stylesheet's text, and it folds the longhands happy-dom expands `outline` into back into one use.

**Hand checks** (on `npm run tauri dev` or the release build), with `core/tests/fixtures/github.md`, in light and dark, on Mica and with the backdrop off (solid), to record here:

1. Links are dark amber in light and light amber in dark, readable on both backgrounds; the Note alert's title and border stay GitHub's blue.
2. A selection is light amber in light and dark amber in dark, and its text, links included, stays readable.
3. Find (Ctrl+F, "lamp"): every match pale amber in light and deep amber in dark, the current one Base with dark text, and the two told apart easily.
4. Tab to a link, the copy button, the find box and (with no document) Open file: each focus ring is amber. Hover a code block's copy button: its icon turns amber.
5. The ticked task item's checkbox: amber, or gray because it is disabled (see Ticks above).
6. High contrast (Windows' contrast themes): links in `LinkText`, matches in `Mark` and `Highlight`, no amber anywhere in the document.
