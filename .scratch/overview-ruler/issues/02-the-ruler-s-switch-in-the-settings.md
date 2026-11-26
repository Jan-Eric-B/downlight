# 02: The ruler's switch in the settings

**What to build:** The reader can switch the **Overview ruler** off and on with an "Overview ruler" checkbox in a new "View" group of the settings flyout. It is on by default, kept in the **Settings** across a restart, and reaches every open window at once. Switching it leaves the view where it was. There is no title bar button, shortcut or right-click entry.

Spec: [Overview ruler](../spec.md)

**Blocked by:** 01 (Heading marks beside the scroll bar)

**Status:** ready-for-agent

- [x] The app's settings store keeps the ruler's switch, defaulting to on when the settings file lacks it; the settings snapshot sent to every page carries it.
- [x] A new setting change sets it (set, not switched, like the flyout's other choices), and the app sends the new snapshot to every window.
- [x] Contract samples are written for the new setting change and the snapshot's new field (`npm run fixtures`), and the page's contract test drives them through the page.
- [x] The flyout gains a "View" group after Accent color and before Edit with, holding one checkbox, "Overview ruler", which shows the setting and sends a change when the reader toggles it; its focus order works with the flyout's keyboard handling.
- [x] The ruler shows or hides with the setting, leaving the view where it was.
- [x] App tests: the switch survives a restart and defaults to on.
- [x] Page tests (written first): the checkbox shows the setting and sends the change; switching it off hides the strip and on shows it again; a new window starts with the saved choice.
- [x] `npm run check` passes.

## Decisions

- **The setting's shape.** `Settings` and `PageSettings` gain `overview_ruler` (`overviewRuler` in the file, the URL and the `settings` message), and `SettingChange` gains `OverviewRuler { overview_ruler }`, sent as `{ "setting": "overviewRuler", "overviewRuler": true }`, after the pattern of Accent color. Its contract samples are `overviewRulerOn` and `overviewRulerOff`, since the flyout offers both values. The sampled snapshot has the ruler off, away from the default, so a page that ignored the field fails its tests.
- **On by default, also for a broken value.** `Settings` now has a hand-written `Default` (the ruler on, the rest as before), which `#[serde(default)]` takes a missing value from. A value that is there but isn't a boolean (a hand edit such as `"off"`) loads as on too, through a new `or_on` beside `or_default`, since `bool`'s own default is off.
- **The checkbox** is a label around an `<input type="checkbox">`, in a `.choices` row like the radio groups, styled with them (`accent-color`, the focus ring); the flyout's Tab handling already stops at every input that isn't a radio, so it is one stop between Accent color and Edit with. The accent test's list of accent uses names the widened selector.
- **Switching** only sets the ruler's state and redraws its strip at once, with no `keepReadingPosition`: the strip takes no layout, so the view stays exactly where it was in every window (spec, "Placement"), and a landing would snap a view part way through a block to that block's top and could cancel a pending zoom or Full width landing. The ruler hears the switch through `whenSettingChanges`, which already drops snapshots that leave `overviewRuler` as it was, so it needs no guard of its own: a page starting up takes the switch from its first call, and a snapshot that changes another setting never reaches it. While off, a resize or a new document only hides the strip, reading no rectangle.
- **Contract tests for the snapshot** sit in `overview-ruler.test.ts` (the `settings` message sample switches the ruler off; a page opened at the `allOff` URL sample starts without it) rather than in `contract.test.ts`, which is already past 1,000 lines; the setting changes are checked in `contract.test.ts`'s test of every change, which now checks and unchecks the box.
