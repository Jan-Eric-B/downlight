# 35: Edit at the reading position

**What to build:** Edit opens the document in the Editor at the line of the reading position, so the reader lands where they were reading instead of at the top of the file.

Spec: [Downlight v1](../spec.md) (Out of Scope lists "edit at the reading line" for v1.x)

**Blocked by:** 15 (Edit and Choose editor)

**Status:** needs-triage

- [ ] Edit passes the reading position's line number to the Editor when the Editor is known to take one, and opens the file at its top otherwise.
- [ ] The page sends the line with `edit_document`; the app checks it is a line number, never a path or anything it would pass on unchecked.
- [ ] Tests: the line reaches the launch for an Editor that takes one, and an Editor that doesn't is launched as today.

## Comments

**2026-11-16, from the reader's hand check of ticket 15.** Ctrl+E opens the file at its top, not at the place shown in the viewer. The spec puts this in v1.x, so it needs triage: in v1 or after it.

Questions for triage:

- Each Editor takes a line differently, and some take none: VS Code `--goto file:line`, Notepad++ `-n<line>`, and the packaged Notepad and the `.txt` program in general may have no such argument (to be checked in their docs). Which Editors does Downlight know, and how does it recognize them: by the program's file name, or a setting per Editor?
- The `.txt` program starts through the shell's handler API (`open_as_text_file`), which passes no arguments, so the line works only for a chosen Editor or one that Downlight starts by its path.
- The line number is the reading position's block (`reading-position.ts`), which is the line where the block at the top of the view starts in the file.
