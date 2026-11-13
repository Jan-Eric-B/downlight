# 15: Edit and Choose editor

**What to build:** Ctrl+E opens the document in the reader's **editor**: the program they chose, or else the program the system uses for plain text files, or else Notepad. "Choose editor..." picks any program and remembers it. Edit never opens Downlight itself. This ticket also creates the settings file.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 09 (Opening from inside a window)

**Status:** ready-for-agent

- [ ] A settings file in the app's config folder holds the chosen editor. It is created on the first save, and later tickets add their values to it.
- [ ] Ctrl+E opens the document in the chosen editor if one is set and still exists; otherwise in the program the system uses for `.txt` files, started through the shell's handler API so that the packaged Notepad on Windows 11 works; otherwise in Notepad.
- [ ] Edit never opens Downlight.
- [ ] "Choose editor..." opens a file dialog for a program, and the choice is remembered across restarts.
- [ ] Edit and Choose editor... are in the right-click menu.
- [ ] The editor launch lives in the Windows module of the per-OS code.
- [ ] Checked by hand: the packaged Notepad as the `.txt` default, Notepad++ or VS Code as the `.txt` default, a chosen editor, and a chosen editor that was uninstalled afterwards (Edit falls back).

## Plan

**Approach:** Two app feature modules, `settings.rs` and `edit.rs`, plus the Windows editor launch in `app/src/platform/windows.rs`. Settings sit behind one deep module: a `SettingsStore` in managed state, backed by `settings.json` in Tauri's `app_config_dir()` (`%APPDATA%\io.github.jan-eric-b.downlight\settings.json`), with `load` and `update(|s| ...)`, which saves the whole file. It loads in `setup` before any window opens, since the file is tiny and ticket 16 needs the zoom level for new windows. Edit takes only the calling window, looks up its document through `OpenWindows::document_path`, works out the editors to try with a pure `editor_order` function, and on a blocking thread tries each until one starts: the chosen `.exe` if it is still a file, then the `.txt` program, then `%SystemRoot%\System32\notepad.exe`. The `.txt` program starts through `ShellExecuteExW` with `SEE_MASK_CLASSNAME`, `lpClass=".txt"` and the default verb, so the shell resolves UserChoice and packaged (MSIX) activation itself and Windows 11 Notepad works. "Never Downlight": a candidate is skipped when its exe, canonicalized and passed through `platform::fold_case`, equals Downlight's own `current_exe()`; for `.txt` the exe comes from `AssocQueryStringW(ASSOCSTR_EXECUTABLE, ".txt", "open")`. When that query gives no exe (no association, or some packaged apps), the shell launch is still tried, since Downlight itself is never packaged.

**Tests first:**

- settings (`cargo test`, temp folders):
  - `missing_file_loads_defaults_and_creates_no_file`
  - `first_save_creates_the_file_and_its_folder`
  - `saved_editor_loads_after_a_restart` - a new store on the same path returns the editor
  - `corrupt_file_loads_defaults_and_reports_why` - `SettingsError::Parse` naming the path
  - `corrupt_file_is_kept_as_bad` - after loading a corrupt file, its bytes are in `settings.json.bad` (an older `.bad` is replaced) and the next save writes a fresh `settings.json`
  - `file_without_an_editor_loads_with_none` - `#[serde(default)]`, so later tickets can add fields
  - `file_with_unknown_fields_keeps_known_values` - a file from a newer version still loads
  - `failed_save_keeps_the_change_for_this_session` - the path is a folder; the call errors and `editor()` returns the new value
- edit (pure `editor_order`, `is_downlight`, `check_choice`):
  - `chosen_editor_that_exists_comes_first`
  - `chosen_editor_that_no_longer_exists_falls_back_to_the_text_file_program`
  - `no_chosen_editor_tries_the_text_file_program_then_notepad`
  - `chosen_editor_that_is_downlight_is_skipped`
  - `text_file_program_that_is_downlight_is_skipped` - the order is then just Notepad
  - `unknown_text_file_program_is_still_tried` - packaged Notepad
  - `own_exe_in_another_case_is_downlight` and `another_program_is_not_downlight` - real files, the test binary's `current_exe`
  - `choosing_downlight_is_refused` and `choosing_a_file_that_is_not_an_exe_is_refused`
  - `edit_without_a_document_says_so`
- platform/windows: `text_file_program_is_an_existing_file_or_none` - the query only, nothing launched.
- page `commands.test.ts`: "Ctrl+E invokes edit_document"; "a failed edit shows a notice"; "the menu lists Edit with Ctrl+E and Choose editor... without a shortcut"; "a second Choose editor before the dialog closes opens no second dialog".

**Changes:**

1. `app/Cargo.toml` - `cargo add serde_json -p downlight` (MIT OR Apache-2.0, already in the lockfile through Tauri, so cargo-deny is unaffected). windows-sys features `Win32_UI_Shell`, `Win32_System_Com`, `Win32_System_Registry` (`SHELLEXECUTEINFOW` is gated on Registry). No Tauri plugin: rfd already gives the dialog, std the file I/O.
2. `app/src/settings.rs` (new) - `Settings { editor: Option<PathBuf> }` (serde `default`, camelCase); `SettingsStore { path: Option<PathBuf>, current: Mutex<Settings> }` with `load(path) -> (SettingsStore, Option<SettingsError>)` (a missing file gives defaults silently; a file that doesn't parse is renamed to `settings.json.bad`, replacing an older one, and the error says so), `editor()`, and `update(change) -> Result<(), SettingsError>` (applies the change under the lock, `create_dir_all`, writes pretty JSON to `settings.json.tmp`, `rename`s it over). `SettingsError`: `NoConfigFolder`, `Read`, `Parse`, `Write`, each with the path and cause, with `Display`.
3. `app/src/main.rs` - `mod settings; mod edit;`. In `setup`, before `open_launch`: `app.path().app_config_dir()`, `SettingsStore::load`, `app.manage(..)`. A load error goes to `platform::show_error` on a spawned thread, as the launch report does: "Downlight's settings in {path} can't be read: {cause}. Defaults are used, and the file was kept as settings.json.bad." (A file that can't be read at all, rather than parsed, is left in place and the message says the next change replaces it.) Register `edit::edit_document` and `edit::choose_editor`.
4. `app/src/window.rs` - `OpenWindows::document_path(label) -> Option<PathBuf>`, the same accessor ticket 11 plans; whichever lands first adds it.
5. `app/src/edit.rs` (new):
   - `pub enum Editor { Chosen(PathBuf), TextFileProgram, Notepad }`
   - `fn editor_order(chosen, is_file: impl Fn, text_file_program: Option<&Path>, is_downlight: impl Fn) -> Vec<Editor>`
   - `fn is_downlight(program, own_exe)`
   - `fn check_choice(program) -> Result<(), EditError>` - `.exe` only (which also avoids Rust's batch-file argument quoting), never Downlight
   - `edit_document(window)` - `spawn_blocking`, tries each editor in order, keeps the last error
   - `choose_editor(window)` - `rfd::FileDialog` parented to the window, titled "Choose editor", filter "Programs" / `exe`, on `spawn_blocking`; nothing chosen changes nothing; a choice goes through `check_choice`, then `update`
   - `EditError`: `NoDocument`, `ChoseDownlight`, `NotAProgram`, `NoEditorStarted { document, source }`, `NotSaved(SettingsError)` ("used until Downlight closes"), `TaskStopped`; `Display` and `Serialize` as text, like `OpenError`
6. `app/src/platform/windows.rs` and `mod.rs`:
   - `text_file_program() -> Option<PathBuf>` - `AssocQueryStringW` with `ASSOCF_INIT_IGNOREUNKNOWN | ASSOCF_NOTRUNCATE`
   - `launch_editor(&Editor, document) -> io::Result<()>` - `Chosen` and `Notepad` through `std::process::Command::spawn`; `TextFileProgram` inside the shared `with_com` helper, then `ShellExecuteExW` with `SEE_MASK_CLASSNAME | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI` and `SW_SHOWNORMAL`. `NOASYNC` because a pool thread has no message loop; `NO_UI` turns a missing association into `ERROR_NO_ASSOCIATION`, which falls back to Notepad.
   - `notepad()` - the path under `SystemRoot`
   - Every `unsafe` block gets a SAFETY comment. Non-Windows fallbacks: `text_file_program` returns None, `Chosen` is spawned, the rest return `Unsupported`.
7. `page/src/commands.ts` - the shared `Command` shape (see below). Add `edit` (Ctrl+E, `invoke("edit_document")`, a notice on error) and `chooseEditor` (no shortcut, an in-flight guard like `opening`). Menu lines "Edit" and "Choose editor..." in the spec's order; `menuEntries` omits the shortcut when there is none.
8. `page/src/commands.test.ts` - the page tests above.
9. `GLOSSARY.md` - a **Settings** entry: the reader's choices remembered across restarts (the editor, later zoom and window size), with no settings window. Code says "text file program", never "handler".
10. No capability change: these are the app's own commands, so no `cargo clean -p tauri --release` is needed.

**Fits with other tickets:**

- `Command` in `page/src/commands.ts` becomes `{ ctrlLetter?: string; key?: string; run(): void }`; the menu shows `key` if set, else `Ctrl+<letter>`, else no shortcut. Ticket 10 needs `key` for F5; whichever of 10, 14 and 15 lands first makes the change.
- Menu order follows the spec: Back, Forward, Reload, Find, Edit, Choose editor..., Open, Zoom, Copy.
- `with_com` (`CoInitializeEx` apartment-threaded with `COINIT_DISABLE_OLE1DDE`, `CoUninitialize` only if init succeeded) is shared with ticket 11's reveal in Explorer, and the windows-sys features are added once.
- Ticket 16 extends `Settings` with zoom and window size through `update`.

**Verification:** `npm run check`, then `npm run tauri build` and by hand on the release build: the packaged Notepad as the `.txt` default; Notepad++ and VS Code as the `.txt` default; Choose editor... picks a program, Edit uses it, and the choice survives a restart (`settings.json` appears only at that first save); uninstall or rename the chosen editor and Edit falls back to the `.txt` program; Downlight as the `.txt` default makes Edit open Notepad; choosing `downlight.exe` is refused with a notice; a corrupt `settings.json` shows the message box and Downlight still starts; Edit in an empty window says there is no document; both menu items show, with Ctrl+E beside Edit; Edit doesn't freeze the window.

**Decisions (2026-11-12):**

- A settings file that doesn't parse is kept as `settings.json.bad` (replacing an older one), so a hand-edited file with a typo isn't lost, and the message says where it went.
- The tickets are built one at a time, so each shared piece in "Fits with other tickets" is added by the first ticket that needs it.

## Comments

**2026-11-12, from ticket 10's code quality review.** What ticket 10 left in place that this ticket builds on:

- `Command` already has `ctrlLetter?` and `key?` (from ticket 10), so Choose editor... can have no shortcut, and the menu shows none for it.
- Put the app side in feature modules of its own (`app/src/edit.rs`, `app/src/settings.rs`) and keep `window.rs` to the window table. Ticket 10 moved Reload into `app/src/reload.rs` for the same reason. `window.rs`'s tests are in `app/src/window/tests.rs`.
- Messages to the reader go through `notice::show_notice` (`app/src/notice.rs`). A failure to launch the editor answers a command, so it can come back as that command's error, as `open_document`'s errors do.
- Code in `app/src/platform/windows.rs` that sets up an OS hook shouldn't fail a window when the hook can't be added. Ticket 10's review found `leave_reload_keys_to_the_page` failing `open_window` after the window was built, which left a visible window without a document.
- Two of ticket 10's review findings apply here too: a return value only the tests read, and the same value worked out in two places. Test the effect a caller sees, and pass a value that is computed once.

**2026-11-13, from the architecture review.** The plan above names code that has moved since 2026-11-12:

- `OpenWindows::document_path` exists already as `Documents::document_path` (`app/src/documents.rs`), so plan item 4 is done.
- `app/src/reload.rs` folded into `app/src/documents.rs` (ticket 22).
- `notice::show_notice` and `app/src/notice.rs` are gone. Messages to a page go as `Message::Notice` through `Pages` (`app/src/message.rs`). An Edit failure still comes back as the command's error, as the plan says.
- `leave_reload_keys_to_the_page` is now `leave_reload_and_find_keys_to_the_page`, and ticket 29 removes it.
- The `Command` shape note under "Fits with other tickets" is superseded by ticket 29, which widens `Command` to a key with its modifiers. If this ticket lands first, add Ctrl+E in today's shape (`ctrlLetter: "e"`).
- If ticket 27 has landed, `edit_document` and `choose_editor` are declared in its page module, not called with string names.
