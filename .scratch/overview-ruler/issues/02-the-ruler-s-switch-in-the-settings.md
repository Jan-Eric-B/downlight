# 02: The ruler's switch in the settings

**What to build:** The reader can switch the **Overview ruler** off and on with an "Overview ruler" checkbox in a new "View" group of the settings flyout. It is on by default, kept in the **Settings** across a restart, and reaches every open window at once. Switching it keeps the reading position. There is no title bar button, shortcut or right-click entry.

Spec: [Overview ruler](../spec.md)

**Blocked by:** 01 (Heading marks beside the scroll bar)

**Status:** ready-for-agent

- [ ] The app's settings store keeps the ruler's switch, defaulting to on when the settings file lacks it; the settings snapshot sent to every page carries it.
- [ ] A new setting change sets it (set, not switched, like the flyout's other choices), and the app sends the new snapshot to every window.
- [ ] Contract samples are written for the new setting change and the snapshot's new field (`npm run fixtures`), and the page's contract test drives them through the page.
- [ ] The flyout gains a "View" group after Accent color and before Edit with, holding one checkbox, "Overview ruler", which shows the setting and sends a change when the reader toggles it; its focus order works with the flyout's keyboard handling.
- [ ] The ruler shows or hides with the setting inside `keepReadingPosition`.
- [ ] App tests: the switch survives a restart and defaults to on.
- [ ] Page tests (written first): the checkbox shows the setting and sends the change; switching it off hides the strip and on shows it again; a new window starts with the saved choice.
- [ ] `npm run check` passes.
