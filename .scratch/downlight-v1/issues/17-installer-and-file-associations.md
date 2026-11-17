# 17: Installer and file associations

**What to build:** A per-user installer that needs no admin rights puts Downlight on a work machine, registers it for the six Markdown extensions so that it appears in "Open with" and in Default apps, and adds it to the PATH. It uses the system WebView2 only and removes everything again on uninstall.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 08 (One process, a window per document)

**Status:** closed

- [x] Tauri's NSIS installer in per-user mode installs without admin rights.
- [x] `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn` and `.mdwn` are associated for the current user, and Downlight appears in "Open with" and in Default apps for them.
- [x] The install folder is added to the user's PATH, so `downlight <file>` works in a new shell.
- [x] WebView2 install mode is "skip": no runtime is bundled or downloaded.
- [x] The identifier is `io.github.jan-eric-b.downlight` and the product name is Downlight.
- [x] The installer is unsigned, with no updater and no network requests of Downlight's own.
- [x] Uninstalling removes the app, its file associations and its PATH entry.
- [x] With Downlight set as the default app, double-clicking a `.md` file in Explorer opens it in Downlight, and a second double-click opens a second window in the same process.

## Results

Built 2026-11-16. `npm run check` passes, and `npm run tauri build` writes the installer to `target/release/bundle/nsis/Downlight_0.1.0_x64-setup.exe` (2.77 MiB) beside `target/release/downlight.exe`. The installer hasn't been run yet.

- **The bundle** (`app/tauri.conf.json`): `bundle.active` is on with `targets: ["nsis"]`, so `npm run tauri build` now also builds the installer (the first run downloads Tauri's NSIS 3.13 and its `nsis_tauri_utils` plugin to `%LOCALAPPDATA%\tauri\NSIS`; the bundler does that, not Downlight). `bundle.windows.nsis` has `installMode: "currentUser"` (installs to `%LOCALAPPDATA%\Downlight`, writes only under HKCU) and `installerHooks: "installer/hooks.nsh"`. WebView2 stays `skip`; the identifier and product name were already right. No signing settings, no updater plugin, no `createUpdaterArtifacts`.
- **Publisher.** Without `bundle.publisher`, Tauri takes the identifier's second part, so Installed apps and the exe's company name would have said "github". It is now `Jan-Eric-B`, which also names the installer's registry folder (`HKCU\Software\Jan-Eric-B\Downlight`).
- **File associations** (`app/installer/hooks.nsh`, after Tauri's install and uninstall). Downlight registers as one of the programs for each extension and leaves the choice of default to the reader: the ProgID `Downlight.Markdown` (described as "Markdown document", the exe's icon, `"downlight.exe" "%1"`), an entry in each extension's `OpenWithProgids` ("Open with"), and `Capabilities` with `FileAssociations` under `HKCU\Software\Jan-Eric-B\Downlight`, named in `HKCU\Software\RegisteredApplications` (Default apps). The shell is told with `SHChangeNotify`. Tauri's own `bundle.fileAssociations` is not used: it overwrites each extension's default ProgID, writes no `OpenWithProgids` or `Capabilities` (so Downlight wouldn't appear in Default apps), and its uninstall writes the old default back over whatever the reader chose since and leaves a `Downlight.Markdown_backup` value in every extension's key. Uninstalling deletes Downlight's values and keys, and an extension's key only if nothing else is left in it.
- **PATH.** The install folder is appended to the user's PATH (`HKCU\Environment\Path`, kept as `REG_EXPAND_SZ`), unless it is already there, and removed again on uninstall, leaving every other entry as it was; a PATH that held only Downlight is deleted. Running shells are told with `WM_SETTINGCHANGE`. NSIS strings hold at most `NSIS_MAX_STRLEN` characters (1024 unless NSIS was built for longer ones), and `ReadRegStr` gives an empty string for a longer value, which is how installers wipe PATHs. So the hooks tell a missing PATH from one too long to read, and when the PATH is too long to read whole, or would be after adding the folder, they leave it alone and tell the reader which folder to add (or remove) by hand.
- **Updates.** The uninstall hook leaves the associations and the PATH entry alone while `$UpdateMode` is 1, as Tauri's own uninstall keeps the shortcuts then. In Tauri CLI 2.12.4's template `$UpdateMode` is 1 only with `/UPDATE` on the command line, which Tauri's updater passes (Downlight has none), and then the installer doesn't run the old uninstaller at all. A plain install over an existing one asks whether to uninstall first; that uninstall is a whole one, after which the install hook writes the associations and the PATH entry again (at the PATH's end).
- **Settings and data.** The identifier hasn't changed since ticket 01, so the settings file stays in `%APPDATA%\io.github.jan-eric-b.downlight\`. Tauri's uninstaller has a "Delete the application data" box, which removes that folder and `%LOCALAPPDATA%\io.github.jan-eric-b.downlight\` (WebView2's data).
- **Grammar licenses** (ticket 07's comment). Every license two-face lists for its syntax grammars is MIT, BSD-2-Clause, BSD-3-Clause or Apache-2.0, and its other types (Unlicense, WTFPL, Sublime's permissive grant) are compatible with GPL-3.0 too. They are written to `page/public/grammar-licenses.md`, which Vite copies to `page/dist/` beside `licenses.md`, so they ship inside the exe with the other notices (186 KB, uncompressed, since Tauri's compression feature is off). Only the grammars are listed: the core uses none of two-face's themes. `npm run fixtures` writes the file again.

**Tests:**

- `app/tests/installer.rs`: `installer_registers_every_markdown_extension_and_no_other` checks the hooks' extension list against the core's `MARKDOWN_EXTENSIONS`; `installer_registers_the_associations_only_through_its_hooks` checks that the config has no `fileAssociations` and names the hooks.
- `core/tests/grammar_licenses.rs`: `every_grammar_license_is_compatible_with_gpl_3` fails on a license type not yet checked; `grammar_licenses_ship_as_committed` fails when two-face lists other licenses than the committed file; `write_grammar_licenses` (ignored) is run by `npm run fixtures`.

**Not yet checked by hand** (the installer above, on a standard user account):

- The installer runs without a UAC prompt and installs to `%LOCALAPPDATA%\Downlight`; Downlight is in the Start menu and in Installed apps, with Jan-Eric-B as its publisher.
- Right-click a `.md` file (and one of each other extension), Open with: Downlight is listed with its icon. Settings, Apps, Default apps: Downlight is listed, and its page offers all six extensions.
- Set Downlight as the default for `.md`, double-click a `.md` file in Explorer: it opens in Downlight. Double-click a second one: a second window opens in the same process (one `downlight.exe` in Task Manager).
- In a new terminal (opened after installing), `downlight <file>` opens the file and returns at once; `$env:Path` (or `echo %PATH%`) ends with the install folder, and the PATH's other entries are unchanged.
- Uninstall from Installed apps: the install folder, the Start menu entry, Downlight in "Open with" and in Default apps, and the PATH entry are gone (in a new terminal), and the other entries of the PATH are unchanged. With "Delete the application data" ticked, the settings folder is gone too. Installing again restores everything.
- The PATH cases, each checked in `HKCU\Environment` (`reg query HKCU\Environment /v Path`) before and after, with every other entry unchanged:
  - A PATH that already holds the install folder (with and without a trailing backslash, in other letter case): installing doesn't add it twice.
  - A PATH ending in `;`: the folder is appended without an empty entry before it.
  - A PATH longer than 1024 characters: installing says to add the folder by hand and leaves the PATH exactly as it was; uninstalling says to remove it by hand and leaves it too.
  - A PATH whose only entry is Downlight's: uninstalling deletes the `Path` value, and a new shell still works from the system PATH.
  - An update, installing the same or a newer version over an existing install: afterwards Downlight is still in "Open with" and Default apps, the reader's default for `.md` still opens Downlight, and the PATH holds the install folder once.

## Comments

**2026-11-10, from ticket 07.** The exe now embeds bat's grammars (two-face). Their licenses (`two_face::acknowledgement::listing()`) still need listing, checking against GPL-3.0 and shipping with the other notices.

**2026-11-16, decided by the repo owner.** The publisher stays `Jan-Eric-B`. The Rust crates' notices and Downlight's own `LICENSE`, which ship nowhere yet, are ticket 36. CLAUDE.md's Commands now name the installer `npm run tauri build` writes.

**2026-11-16, from the review.** The uninstall hook removed the ProgID, the Capabilities, the RegisteredApplications value and the PATH entry even while `$UpdateMode` was 1. It now does all of that only when `$UpdateMode <> 1`, as Tauri's template guards the shortcuts (checked in the template inside `@tauri-apps/cli` 2.12.4, see Updates in the Results), and the `DownlightUpdating` variable, which only kept the PATH message quiet during an update, is gone. `npm run tauri build` still builds the installer; it hasn't been run. The hand checks now list the PATH cases: one that already holds the folder, one ending in `;`, one longer than 1024 characters, uninstalling when Downlight is the only entry, and an update keeping the associations and the PATH entry.

**2026-11-17, checked by hand and closed.** The reader went through every hand check above on the release build of commit f325a45 (test documents in `target/hand-checks/`) and found them fine. One thing seen on the way, a window of a multi-document launch once showing white text on a white backdrop, is unrelated to this ticket and became window-design ticket 13.
