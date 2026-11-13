# 05: Open and Edit buttons, and the separators

**What to build:** The title bar gets its button row: Open and Edit, then a separator, then the slots for Full width, Keep on top and Settings, then a separator before minimize, maximize and close. Open and Edit run the same commands as Ctrl+O and Ctrl+E.

Spec: [Window design](../spec.md)

**Blocked by:** 01 (Spike: the page draws the title bar), v1 [15](../../downlight-v1/issues/15-edit-and-choose-editor.md) (Edit and Choose editor)

**Status:** ready-for-agent

- [ ] Open and Edit are buttons with Fluent UI System Icons, and run the commands of the page's command table, not a copy of them.
- [ ] Their tooltips name the command and its shortcut ("Open (Ctrl+O)", "Edit (Ctrl+E)"), taken from the command table.
- [ ] Thin separators set apart Open and Edit; Full width, Keep on top and Settings; and the window's own buttons, as the spec's table shows.
- [ ] Edit is disabled in a window without a document.
- [ ] The buttons are reachable with Tab and have accessible names.
- [ ] Page tests: each button runs its command; the tooltip carries the shortcut; Edit is disabled without a document.
