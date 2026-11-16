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
