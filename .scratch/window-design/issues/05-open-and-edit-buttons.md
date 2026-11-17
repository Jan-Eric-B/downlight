# 05: Open and Edit buttons, and the separators

**What to build:** The title bar gets its button row: Open and Edit, then a separator, then the slots for Full width, Keep on top and Settings, then a separator before minimize, maximize and close. Open and Edit run the same commands as Ctrl+O and Ctrl+E.

Spec: [Window design](../spec.md)

**Blocked by:** 01 (Spike: the page draws the title bar), v1 [15](../../downlight-v1/issues/15-edit-and-choose-editor.md) (Edit and Choose editor)

**Status:** ready-for-human

- [x] Open and Edit are buttons with Fluent UI System Icons, and run the commands of the page's command table, not a copy of them.
- [x] Their tooltips name the command and its shortcut ("Open (Ctrl+O)", "Edit (Ctrl+E)"), taken from the command table.
- [x] Thin separators set apart Open and Edit; Full width, Keep on top and Settings; and the window's own buttons, as the spec's table shows. (Built; their look is hand check 1.)
- [x] Edit is disabled in a window without a document.
- [x] The buttons are reachable with Tab and have accessible names. (Tested; the focus ring is hand check 3.)
- [x] Page tests: each button runs its command; the tooltip carries the shortcut; Edit is disabled without a document.

## Comments

**2026-11-16, from v1 ticket 15.** Edit is the `edit` entry of the command table (`page/src/commands.ts`, Ctrl+E), which runs `edit` in `page/src/edit.ts`, so the button is `commandButton("edit", glyph)`. The entry has no `available` yet: in a window without a document the app rejects `edit_document` with "There is no document to edit.", which shows as a notice. This ticket's "Edit is disabled without a document" adds `available` to the entry, which then also greys Edit out in the right-click menu.

**2026-11-16, from the design review of v1 tickets 12 to 15.** Give the command table state on its buttons here, before 06 and 07 need it: an optional `pressed?()` on `Command` beside `available`, and one `refreshCommands()` that sets `disabled` and `aria-pressed` on every command button, run on `whenDocumentShown`, on a settings change (ticket 12) and on `window-state`. Edit without a document is then `available`, kept current, rather than read only when the right-click menu opens; Full width and Keep on top become one table entry and a glyph each.

**2026-11-17, built; the hand checks are left.** What was built, and what it decided:

- **The command table has button state now**, as the design review asked. `Command` has an optional `pressed?()` beside `available?()`. `commandButton` marks its button with `data-command`, sets `disabled` from `available` and, for a command with `pressed`, `aria-pressed`, when it makes the button; `refreshCommands()` in `page/src/commands.ts` sets both again on every command button in the page. It runs on `whenDocumentShown` and on every settings change, both from the title bar (`startTitleBar`, which owns the buttons); Keep on top (ticket 07) can run it after the app's answer. Not on `window-state`: nothing a button shows depends on maximize or restore. The right-click menu reads `available` and `pressed` when it opens, so it needs no refresh, and a command with `pressed` gets a check mark there (`checked`, ticket 06).
- **Edit's `available`** is `showsDocument()` from `page/src/view.ts`, true while the view has a document's file, so also for a document whose file can't be read: the app still has its path, and the editor is where a reader would mend it. It greys Edit out in the right-click menu too. Ctrl+E still asks the app in an empty window, which answers with its notice ("There is no document to edit."), so the key isn't silently dead.
- **The buttons** are `commandButton("open", ...)` and `commandButton("edit", ...)` with the 16 px regular glyphs `folder_open` and `edit`, in a `.command-buttons` row between the drag strip and the window's buttons. Groups are a list in `createCommandButtons` (`page/src/title-bar.ts`), each followed by a separator, so the second group (Full width, Keep on top, Settings) and the separator before the window's buttons need no more code; tickets 06 and 07 each added one entry. Separators are `aria-hidden` lines, 1 px by 16 px, in WinUI's divider colors (8% black, 8.4% white), `GrayText` in high contrast.
- **The tooltip drops the menu label's ellipsis**: Open's label is "Open..." for the menu, and its button says "Open (Ctrl+O)", as Windows' own tooltips do. `commandButton` cuts a trailing "...", so the label stays one entry in the table.
- **Focus:** unlike the window's buttons, Open and Edit are in the Tab order: they have no system key as Alt+Space, and the ticket asks for it. A click still leaves the focus in the document (the bar's `mousedown` is prevented). Their focus ring is the accent's emphasis role, a new row in `accent.test.ts`'s use table. A disabled Edit takes the inactive bar's color and no hover fill.
- **Size:** 32 by 28 px with a 4 px rounded hover fill, as the file name's, so they read as the app's commands and not the window's (46 by 32 px, square). If the hand check finds them cramped or too airy, the numbers are in `page/src/title-bar.css`.
- **Tests** (`page/src/title-bar.test.ts`, written first and failing): the row's groups and separators, Open and Edit run the same commands as Ctrl+O and Ctrl+E, the tooltips, Edit disabled without a document and following the shown document both ways, enabled for an unreadable file, Tab reaches both. In `commands.test.ts` the right-click menu's test now starts an empty window's view, so Edit's state no longer depends on what an earlier test left behind, and a new test offers Edit once a document is shown.

**Hand checks** (on the release build, `npm run tauri build`), to record here:

1. The bar reads in three groups: Open and Edit, a thin line, Full width (Keep on top and Settings join it in tickets 07 and 08), a thin line, then minimize, maximize and close. The lines are visible but quiet, in light and dark, on Mica and solid.
2. Open opens the open dialog and Edit the editor, as Ctrl+O and Ctrl+E do; each tooltip names the command and its shortcut; in an empty window (`downlight` with no file) Edit is grey, has no hover, and is grey in the right-click menu too.
3. Tab from the document's start reaches Open, then Edit, each with an amber focus ring; Enter runs it. A click on either leaves Page Down scrolling the document.
4. Dragging between the name and Open moves the window; pressing on a button doesn't. Hovering maximize still shows the snap layouts flyout.
5. A real contrast theme: the glyphs, the separators and a disabled Edit stay legible.

**2026-11-17, review fixes.** The command table now decides when its buttons show their state again, rather than the title bar: `startCommands` in `page/src/commands.ts` runs the refresh on `whenDocumentShown` and on every settings change, removed when its signal aborts, and `refreshCommands` is private again (Keep on top still runs it after the app's answer, since nothing else tells the page). `startTitleBar` no longer imports it, so a later command with state from elsewhere can't be left out by a title bar that happens not to know what it follows. `startCommands` therefore runs after `startSettings`, as in `main.ts`. The title bar's zoom level no longer mirrors the zoom and whether a document is shown in locals of its own: `createZoomLevel` has a no-argument `refresh` that reads `settingsNow().zoom` and `showsDocument()` from `view.ts`, run from both listeners, and hides at `DEFAULT_SETTINGS.zoom`, the level Reset gives (`UNZOOMED` is gone). Tests changed: `title-bar.test.ts`'s three Edit tests start commands, as the page does, since the refresh moved there; `commands.test.ts`, `find.test.ts`, `history.test.ts`, `ticks.test.ts`, `zoom.test.ts` and four contract tests start the settings before the commands.
