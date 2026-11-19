# 09: Windows' accent color as a choice

**What to build:** The settings flyout's Accent color offers Windows' accent color next to Amber. Choosing it swaps the accent scale for Windows' seven shades, and the colors follow when Windows' accent changes. The logo and the icon stay amber.

Spec: [Window design](../spec.md)

**Blocked by:** 03 (The accent color in the page), 08 (The settings flyout)

**Status:** ready-for-agent

- [ ] The Windows module reads Windows' accent shades (`UISettings.GetColorValue`, AccentLight3 to AccentDark3 and Accent) and reports when they change (`ColorValuesChanged`).
- [ ] With Windows' accent chosen, every accented use shows the matching shade, in light and dark, by the same shade-to-use table as amber.
- [ ] Changing the accent in Windows' settings recolors every open window without a reload.
- [ ] The title bar logo stays amber.
- [ ] Other OSes report no accent of their own, and the choice is hidden there.
- [ ] A contract sample for the accent message, and page tests that a scale from the app replaces the variables.
- [ ] Checked by hand with a light and a dark Windows accent, in light and dark, checking that links stay legible.

## Comments

**2026-11-16, from ticket 02's review.** Suggestions, not decisions:

- Set Windows' seven shades inline on `document.documentElement` only. Custom properties inherit as computed values, so roles declared on `:root` from the shades (ticket 03) won't change if the shades are set on an element lower down. For example an `accent.ts` with `applyAccent(scale | null)`, where `null` removes the inline shades and amber from `accent.css` shows again.
- Have a test check that the accent message's contract sample keys match the shade names `accent.css` defines, and make `accent.test.ts`'s `SHADES` list come from that one production list, so the names aren't defined in four places (the CSS, the message type, the sample and the test).

**2026-11-16, from the review of tickets 03 and 04.** Shades (`--accent-light-1`) and roles (`--accent-link`) share the `--accent-` prefix, so setting or removing the inline shades must go by the one shade list, never by a prefix match, or it would touch the roles too.

**2026-11-17, from ticket 08 (the settings flyout).** What the flyout leaves for Windows' accent color:

- The setting exists: `AccentColor` in `app/src/settings/look.rs` (only `Amber`), the field `accent_color` in `Settings` and `PageSettings` (`accentColor` on the page), and `SettingChange::AccentColor { accent_color }`. Adding Windows' is a variant of `AccentColor` (say `Windows`), its sample in `setting_changes.json` (the exhaustive match in `settings::change::contract_samples` stops the build until it has one), and on the page a value in `ACCENT_COLORS` (`page/src/app.ts`) and its label in `ACCENT_COLOR_LABELS` (`page/src/settings-flyout.ts`). The flyout builds the group from that list, so the control needs nothing else.
- The page follows the choice with `whenSettingChanges("accentColor", ...)` (`page/src/settings.ts`), as `backdrop.ts` follows the backdrop; the shades themselves come from the app's accent message.
- "The choice is hidden there" on other OSes: the page has no flag for whether the OS has an accent color of its own. A `PageFlags` flag like `backdropSupported` (a field of `PageFlags` in `app/src/window.rs`, whose struct literal in `contract_samples` stops the build until it is sampled) would do; the flyout then leaves the Windows value out of the group's list, or hides the group, which has one choice left.
- Today Amber is always checked, so its radio never sends a change, and `contract.test.ts` dispatches its `change` to sample it; with two choices that test can click each one instead.
