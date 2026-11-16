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

## Comments

**2026-11-16, from v1 ticket 15.** Edit is the `edit` entry of the command table (`page/src/commands.ts`, Ctrl+E), which runs `edit` in `page/src/edit.ts`, so the button is `commandButton("edit", glyph)`. The entry has no `available` yet: in a window without a document the app rejects `edit_document` with "There is no document to edit.", which shows as a notice. This ticket's "Edit is disabled without a document" adds `available` to the entry, which then also greys Edit out in the right-click menu.

**2026-11-16, from the design review of v1 tickets 12 to 15.** Give the command table state on its buttons here, before 06 and 07 need it: an optional `pressed?()` on `Command` beside `available`, and one `refreshCommands()` that sets `disabled` and `aria-pressed` on every command button, run on `whenDocumentShown`, on a settings change (ticket 12) and on `window-state`. Edit without a document is then `available`, kept current, rather than read only when the right-click menu opens; Full width and Keep on top become one table entry and a glyph each.
