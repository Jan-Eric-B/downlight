# 06: Full width

**What to build:** **Full width** lets the document fill the window instead of GitHub's reading width. A toggle button in the title bar, Ctrl+Alt+W and a right-click menu entry switch it, and the choice is kept in the settings for every window.

Spec: [Window design](../spec.md)

**Blocked by:** 03 (The accent color in the page), 05 (Open and Edit buttons, and the separators), 12 (Settings reach every page)

**Status:** ready-for-agent

- [ ] The button, Ctrl+Alt+W and the menu entry run one command, and the button shows in the accent color while Full width is on.
- [ ] Switching it keeps the reading position.
- [ ] The choice is saved in the settings file and applies to every open window and every new one.
- [ ] Page tests: the command switches the view's width and the button's state; a new window starts with the saved choice.
- [ ] App tests: the settings keep Full width across a restart and default to off.

## Comments

**2026-11-16, decided after the review of tickets 03 and 04.** A switched-on title bar button shows its glyph in `--accent-emphasis` (Dark 1 in light, Light 1 in dark, at least 3:1 as WCAG asks of icons) on the bar's normal hover background, with no accent fill, so the spec's shade table and the five roles stay as they are. When the window isn't active, the switched-on glyph dims with the bar, as Windows dims its own caption.

Suggestion from the same review: `commandButton` in `page/src/commands.ts` has no toggle state, and this ticket and 07 both need one (Maximize already keeps its own `dataset.state`). Give the command table toggles in one place before building either, e.g. `commandButton` returning a `setPressed(on)` that sets `aria-pressed`, styled once in `page/src/title-bar.css` with `.title-bar [aria-pressed="true"]` and its inactive rule.

**2026-11-16, from the design review of v1 tickets 12 to 15.** Full width arrives through the settings snapshot (ticket 12, now a blocker). "Switching keeps the reading position" and v1 ticket 16's CSS zoom both mean "read the line, change the layout, land again": export one `keepReadingPosition(change)` from `page/src/view.ts` that does it with `reading-position.ts`, and let Find hear it as it hears a reload, rather than building it twice.

**2026-11-16, from v1 ticket 16 (Zoom and window size).** `keepReadingPosition(change)` is built in `page/src/view.ts`: it reads the reading position, runs `change`, and puts the view back at it, and of two changes in quick succession only the later one lands. It doesn't tell Find or the `whenDocumentShown` listeners anything, since the view's content stays as it was and Find's matches are ranges in it that stay valid; if Full width's toggle needs Find to hear it (for instance once the overview ruler draws match positions), that is this ticket's call. The settings snapshot of ticket 12 is built too (see its Comments), so Full width is a field of `PageSettings` and a `whenSettingsChange` listener.
