# 09: Windows' accent color as a choice

**What to build:** The settings flyout's Accent color offers Windows' accent color next to Amber. Choosing it swaps the accent scale for Windows' seven shades, and the colors follow when Windows' accent changes. The logo and the icon stay amber.

Spec: [Window design](../spec.md)

**Blocked by:** 03 (The accent color in the page), 08 (The settings flyout)

**Status:** closed

- [x] The Windows module reads Windows' accent shades (`UISettings.GetColorValue`, AccentLight3 to AccentDark3 and Accent) and reports when they change (`ColorValuesChanged`).
- [x] With Windows' accent chosen, every accented use shows the matching shade, in light and dark, by the same shade-to-use table as amber.
- [x] Changing the accent in Windows' settings recolors every open window without a reload.
- [x] The title bar logo stays amber.
- [x] Other OSes report no accent of their own, and the choice is hidden there.
- [x] A contract sample for the accent message, and page tests that a scale from the app replaces the variables. (No message of its own: the scale comes in the `settings` message, sampled with it.)
- [x] Checked by hand with a light and a dark Windows accent, in light and dark, checking that links stay legible.

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

**2026-11-19, built.** What there is now, and what it decided:

- **The choice:** `AccentColor::Windows` (`windows` on the page) beside Amber, in `app/src/settings/look.rs`, sampled in `setting_changes.json` as `accentColorWindows`. The flyout labels it "Windows".
- **The scale:** `AccentScale` in `look.rs`, seven `Rgb` shades that go to the page as `#rrggbb`, keyed by the names `accent.css` gives its variables (`light-3` to `dark-3`). `ACCENT_SHADES` in `page/src/app.ts` is the one list of those names: the page's `AccentScale` type, `isPageSettings`' check, `accent.ts` and `accent.test.ts`'s `SHADES` all come from it.
- **No accent message of its own.** Windows' scale is a field of `PageSettings`, `windowsAccent`, though it is the OS's and not the reader's choice. So it rides what the settings already have: the page's URL for first paint, the `settings` message for a change, and `page_hears_settings`' hold for a page that doesn't hear them yet. The documents module keeps the scale now (`Documents::change_windows_accent`, under the settings' lock, so it orders with a change of the settings); `Settings::page` takes it as an argument. It is sent whatever the reader chose, so choosing Windows' in the flyout needs no round trip for the shades. An unchanged scale sends nothing, since Windows tells of a change of its theme too, and of one change more than once.
- **The Windows module:** `platform::follow_accent(changed)` creates a `UISettings`, listens to `ColorValuesChanged` first and then reads the scale (so a change between the two isn't lost), and keeps the `UISettings` in a static, since an event stops with its source. It runs once in `main`, after the documents module is managed and before `setup` opens the first window. The `windows` crate 0.62.2 with `UI_ViewManagement` is new in `app/Cargo.toml`, the version webview2-com already brings, so nothing joins the tree and the third-party notices are unchanged.
- **Other OSes:** `follow_accent` gives None, so `windowsAccent` is null and the flyout leaves the Accent color group out (a group of one choice is none) instead of a `PageFlags` flag.
- **The page:** `accent.ts` (`startAccent`, in `main.ts` before first paint) sets Windows' shades inline on `document.documentElement` while the choice is Windows', and removes them for amber, by the shade list alone, never by the `--accent-` prefix. The logo stays amber with no change: it is the icon's SVG, which paints with fixed colors (`accent.test.ts` fails on any other).
- **Tests:** the app's: a change of Windows' scale reaches every page and new windows, an unchanged one tells no page, one before a page asks reaches it when it asks, the scale's JSON, a saved Windows choice loads after a restart, and Windows' read itself (seven shades, lightest first). The page's: `accent.test.ts` (Windows' shades in place of amber's, Amber with Windows' scale shows amber, a choice from any window, a new scale recolors without a reload, amber removes the shades alone), the flyout's (both choices, none where the OS has no accent), and the contract's (the flyout clicks each accent choice instead of dispatching `change`, the sampled scale names exactly `ACCENT_SHADES`, and the `settings` sample colors the page with its scale).
- **One test changed:** `page_url_gives_the_page_the_settings_it_starts_with` pins the encoded URL, which gained `windowsAccent: null`.
- **Microsoft's docs disagree** on whether `ColorValuesChanged` fires in a desktop app: the Win32 theming guide subscribes to it, while the list of WinRT APIs desktop apps can't use names it. The hand check settled it (below).

`npm run check` passes (the app's tests 321, the page's 417).

**Measured** on the release build of the working tree (medians of 5, not against `HEAD` built the same way). The scale is read on the main thread before the first window opens, within `windowOpening`, which is 81 ms from the process's start. Cold first paint: 100 KB 687 ms (target under 1.0 s; ticket 02's baseline 771 ms, v1 ticket 28's 787 ms), 1 MB mixed 1102 ms, 1 MB prose 696 ms. A second window, 100 KB: 233 ms from the window opening (target under 300 ms). A reload of 1 MB mixed: 1202 ms from the save, of which the read varied from 120 to 426 ms between runs while the render stayed even; this ticket doesn't touch reloads. Nothing over 100 ms after a document or a reload; typing into Find in 1 MB mixed had single frames of 109 to 118 ms, as before.

**2026-11-20, checked by hand and closed.** The repo owner went through the hand check on that release build and found it fine.

**2026-11-20, reviewed.** A maintainability review found two holes in following Windows' accent, both fixed:

- **A failed read looked like an OS with no accent.** The change handler sent None when one `GetColorValue` failed, so a reader on Windows' accent fell back to amber while the flyout still offered it. Now a failed read keeps the last scale, and `windowsAccent` is null only on an OS that never has an accent color.
- **A race at startup.** `main` stored the scale `follow_accent` had read, after a change told meanwhile could have stored a newer one. Now the platform interface is split: `platform::accent_scale()` reads the scale once, and `platform::follow_accent(changed)` only tells that the colors changed. `Documents::change_windows_accent()` reads the scale itself, under the settings' lock, through the `Windows` port's `accent_scale` (faked by the tests' recorder), so the scale stored last is always the newest. `main` follows first and then reads once. On other OSes `accent_scale` gives None and `follow_accent` does nothing. This replaces the "Windows module" and "Other OSes" notes above.
- New tests: `choosing_windows_accent_color_sends_the_scale_windows_has_now` and `failed_windows_accent_read_keeps_the_scale_the_pages_have`. The platform test reads `accent_scale()` directly and leaves no handler behind. Three tests of the scale's changes now go through a helper, since `change_windows_accent` takes no argument; what they check is unchanged.
