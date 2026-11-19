# 35: Edit at the reading position

**What to build:** Edit opens the document in the Editor at the line of the reading position, so the reader lands where they were reading instead of at the top of the file.

Spec: [Downlight v1](../spec.md) (Edit; moved into v1 from Out of Scope's "edit at the reading line")

**Blocked by:** 15 (Edit and Choose editor)

**Status:** ready-for-human

- [x] Edit passes the reading position's line number to the Editor when the Editor is known to take one, and opens the file at its top otherwise.
- [x] The page sends the line with `edit_document`; the app checks it is a line number, never a path or anything it would pass on unchecked.
- [x] Tests: the line reaches the launch for an Editor that takes one, and an Editor that doesn't is launched as today.
- [ ] Hand checks in the release build (2026-11-19, built, below).

## Comments

**2026-11-16, from the reader's hand check of ticket 15.** Ctrl+E opens the file at its top, not at the place shown in the viewer. The spec puts this in v1.x, so it needs triage: in v1 or after it.

Questions for triage:

- Each Editor takes a line differently, and some take none: VS Code `--goto file:line`, Notepad++ `-n<line>`, and the packaged Notepad and the `.txt` program in general may have no such argument (to be checked in their docs). Which Editors does Downlight know, and how does it recognize them: by the program's file name, or a setting per Editor?
- The `.txt` program starts through the shell's handler API (`open_as_text_file`), which passes no arguments, so the line works only for a chosen Editor or one that Downlight starts by its path.
- The line number is the reading position's block (`reading-position.ts`), which is the line where the block at the top of the view starts in the file.

**2026-11-19, triage, decided by the repo owner.** In v1, and a blocker of ticket 18; the spec's Out of Scope no longer lists it.

- **How an Editor is known:** a built-in table in the app maps a program's file name to how it takes a line. The arguments are Downlight's own, never written by the reader; there is no setting and no UI for them. An Editor not in the table opens the file at its top, as today.
- **The table for v1** (check each program's current docs for the exact flags and file names when building):
  - VS Code (`Code.exe`, `Code - Insiders.exe`): `--goto file:line`.
  - Notepad++ (`notepad++.exe`): `-n<line> file`.
  - Sublime Text (`sublime_text.exe`, `subl.exe`): `file:line`.
  - JetBrains IDEs (`idea64.exe`, `rider64.exe`, `pycharm64.exe`, `webstorm64.exe` and the others): `--line <line> file`.
- **The `.txt` program:** when no Editor is chosen and the `.txt` program's file is in the table, Edit starts it by its path with the line, rather than through the shell. Any other `.txt` program, the packaged Notepad among them, is opened through the shell at the top, as today.

**2026-11-19, built.** Edit opens the document at the reading position's line in an Editor the app knows to take one, as the triage decided.

- **The page** (`page/src/edit.ts`, `page/src/view.ts`, `page/src/reading-position.ts`): Edit sends `edit_document` with `{ line }` (`page/src/app.ts`). The line is the reading position's. At the start of the document, above its first block, it is 1, so an Editor that already has the file open moves to its top too. It is null when the block at the top of the view has no line number (raw HTML), so that Editor opens the file as it does without a line rather than at a wrong one. `lineToEditAt` in `reading-position.ts` gives it, next to `readingPosition`, which stays undefined at the start for Reload, Back and Find.
- **The line in the app** (`app/src/edit/line.rs`): `edit_document` takes `line: Option<Line>`, and `Line` deserializes only from a JSON number that is a positive whole number fitting in 32 bits (`NonZeroU32`). Zero, a negative, a fraction, a larger number, a string or anything else fails the command before it runs, as Tauri can't hand it over. The arguments never take text from the page: the line is formatted from the number, and the document's path, which the app takes from its own record of the window (never from the page), is always one whole argument. An absolute path never starts with `-`, so no Editor reads it as an option, and Rust quotes each argument for the program's command line (`is_program` keeps Edit to `.exe` files, so no batch file's parsing applies).
- **The table** (`KNOWN_EDITORS` in `app/src/edit/line.rs`): the program's file name, compared ignoring ASCII case as Windows does, gives how it takes a line; `editor_arguments` builds the arguments. An unknown program, or no line, gets the document's path alone, as before. No setting and no UI.
- **The `.txt` program** (`editor_order` in `app/src/edit.rs`): a text file program whose file is in the table (and is a full path) is tried as `Editor::TextFileProgramByPath`, started by its path with the line, like a chosen editor. Any other, the packaged Notepad among them, is still started through the shell with no line. This applies whenever Edit gets to the text file program: with no chosen Editor, and also when the chosen one is gone or doesn't start. Edit with's Default still names it after the text file program.
- **The launch** (`app/src/edit.rs`): `launch_of` turns each Editor into a `Launch` (a program and its arguments, or the text file program through the shell), and `start` runs it. `edit` takes the chosen editor and the text file program as `Editors` instead of asking the system itself, so the tests no longer depend on the test machine's `.txt` program.
- **Tests.** Page (`page/src/edit.test.ts`): Ctrl+E sends the line of the block at the top of the view, the innermost block's line, and 1 at the start of the document. App (`app/src/edit/line.rs`): the line is taken from a number or null and refused in every other form; each Editor in the table gets its arguments (VS Code and Insiders, Notepad++, both Sublime programs, every JetBrains name); the file name matches in any case; an unknown program, a folder named like a known one, and no line give the path alone; a path named like options stays one argument. App (`app/src/edit.rs`): a text file program in the table is tried by its path, also after a chosen editor; Edit gives a chosen unknown editor the path alone, VS Code as the text file program `--goto <file>:15`, and an unknown text file program the shell launch; the fallback and failure tests now check launches. One test constant changed: the tests' unknown text file program was `notepad++.exe`, which the table now knows, so it is `EmEditor.exe` now, and Notepad++ is the known one.
- **Contract samples.** No sample changed: the line is a number, which `app/src/contract.rs` doesn't sample, and the contract tests pass as committed.
- **Sources for each Editor's flags**, checked on 2026-11-19:
  - VS Code: `-g`/`--goto` with `file:line[:character]`, [VS Code CLI docs](https://code.visualstudio.com/docs/configure/command-line). The docs only show the `code` command. `code.cmd` (`resources/win32/bin/code.cmd` in microsoft/vscode) runs `Code.exe` with `cli.js` and the same arguments, and `Code.exe` started directly parses the same options in its main process (`parseMainProcessArgv` in `src/vs/platform/environment/node/argvHelper.ts`, which checks each path against `FILE(:LINE(:CHARACTER))` when `goto` is set), so Downlight passes `--goto` to `Code.exe` itself. `Code - Insiders.exe` is the same program under the Insiders name.
  - Notepad++: `-nLineNumber` ("Scroll to indicated line on filepath"), no space, and "the order of the options is not important", [Notepad++ user manual, Command Line](https://npp-user-manual.org/docs/command-prompt/).
  - Sublime Text: "Filenames may be given a :line or :line:column suffix", and on Windows the command line helper is `subl.exe`, [Sublime Text CLI docs](https://www.sublimetext.com/docs/command_line.html). The docs don't name `sublime_text.exe`. That `sublime_text.exe --help` prints the same suffix line comes from the Sublime forum (a reader's Windows build 3126 output), not from the docs, so `sublime_text.exe` is a hand check.
  - JetBrains: `<ide>64.exe [--line <number>] [--column <number>] <path>`, `--line` before the path, from each IDE's "Open files from the command line" page on jetbrains.com/help: IntelliJ IDEA (`idea64.exe`), PyCharm (`pycharm64.exe`), WebStorm (`webstorm64.exe`), Rider (`rider64.exe`), CLion (`clion64.exe`), GoLand (`goland64.exe`), PhpStorm (`phpstorm64.exe`), RubyMine (`rubymine64.exe`), DataGrip (`datagrip64.exe`), DataSpell (`dataspell64.exe`). RustRover's page names the launcher `rustrover.exe`, while its installs have `bin\rustrover64.exe`, so the table has both. The Community editions use the same file names.
- **Found, not built** (for the owner to decide; each would be one line in the table):
  - Editors built on VS Code take `--goto` too: VSCodium (`VSCodium.exe`), Cursor (`Cursor.exe`), Windsurf (`Windsurf.exe`). Android Studio (`studio64.exe`) is built on IntelliJ and takes `--line`. None is in the decided table, and none was checked against its docs.
  - JetBrains Toolbox puts `.cmd` scripts on the PATH, which Choose editor... refuses (`.exe` only, ticket 15); the reader picks the IDE's `bin\<ide>64.exe` instead.

**Hand checks** for the owner, in the release build, each with a document scrolled well down (say to a heading on line 200), then Ctrl+E:

1. **Each Editor you have, as the chosen editor** (Choose... under Edit with in the settings flyout, then the program's `.exe`): VS Code (`Code.exe`, and `Code - Insiders.exe` if installed), Notepad++, Sublime Text (`sublime_text.exe`, and `subl.exe` beside it), and each JetBrains IDE (`bin\<ide>64.exe`). Each opens the document with the cursor on the heading's line. Try once with the Editor closed and once with it already running with the file open in it at another line: the cursor moves to the line both times.
2. **At the start of the document:** scroll to the top, Ctrl+E: the Editor goes to line 1.
3. **An Editor not in the table**, such as `C:\Windows\System32\notepad.exe` chosen: it opens the document at its top, as before.
4. **No chosen Editor** (Default in Edit with), with the `.txt` program set in Windows to Notepad++ or VS Code: Ctrl+E opens the document at the line. Then with the `.txt` program set to the packaged Notepad: it opens through the shell at the top, as before; and with Downlight as the `.txt` program, Edit falls back to Notepad.
5. **A path with spaces and non-ASCII letters**, such as `C:\Temp\Mein Ordner\Übersicht.md`, in VS Code, Notepad++ and a JetBrains IDE: the right file opens at the line, with no extra file or window.

**2026-11-19, review fixes.**

- **How the text file program starts is decided at the launch, not in the editor order** (`app/src/edit.rs`). `Editor::TextFileProgramByPath` is gone: `Editor::TextFileProgram(Option<PathBuf>)` carries the program's file, or None when the system names none, and `editor_order` and `editor_choices` read as they did before this ticket. `launch_of` starts the text file program by its path with the line when its file is named in full, is in the table and is there (`is_file`, as a chosen editor is checked); otherwise through the shell at the top, as before. So a registry entry naming an uninstalled editor now goes through the shell, rather than failing to start and falling to Notepad. `edit` takes `is_file` beside `start`, so the tests choose which programs are there. Tests: `text_file_program_that_takes_a_line_but_is_gone_opens_through_the_shell` (red first), `text_file_program_that_takes_a_line_opens_at_it_after_a_chosen_editor_that_does_not_start`, which replaces the two editor order tests of the by-path variant.
- **The guard against a document named like an option is checked** (`app/src/edit/line.rs`): `editor_arguments` has `debug_assert!(document.is_absolute())`, since an absolute path, which never starts with `-`, is the whole guard. Every test already passes absolute paths. The window's record takes the path from the launch (made absolute against the launching folder), the dialog, a drop or a link, all absolute; only a launch whose current folder can't be read could keep a relative one.
- **The null line is tested** (`page/src/edit.test.ts`): `Edit in raw HTML, which has no line number, sends no line` lays out `find.html` with its raw HTML `<summary>` at the top of the view and expects `{ line: null }`. It fails with the line of the paragraph inside the `<details>` when `lineToEditAt` falls back to a number. `layOut` (`page/src/test-support.ts`) takes the lines of raw HTML elements for this, as only the file knows them. The tests now each show their document (`show`), since this one shows `find.html`.
- **The glossary leaves the list of editors to the code.** The Editor entry says "One that Downlight knows by its program's file name" instead of naming the editors; the Text file program entry adds "and its file is there". The spec's Edit bullet says the same.
