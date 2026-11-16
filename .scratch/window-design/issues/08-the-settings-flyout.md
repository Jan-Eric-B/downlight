# 08: The settings flyout

**What to build:** The Settings button (and Ctrl+,) opens a flyout under it with Theme, Backdrop, Accent color and Edit with. A change applies at once to every open window and is kept in the settings. "Choose editor..." leaves the right-click menu, and Settings joins it, last.

Spec: [Window design](../spec.md)

**Blocked by:** 05 (Open and Edit buttons, and the separators), v1 [15](../../downlight-v1/issues/15-edit-and-choose-editor.md) (Edit and Choose editor), 12 (Settings reach every page)

**Status:** ready-for-agent

- [ ] The flyout is drawn by the page under the gear, above the document, and doesn't cover the whole window.
- [ ] Esc, a click outside and the gear again close it. While it is open, Tab moves through its controls and the focus stays inside; on closing, the focus returns to the gear.
- [ ] **Theme:** System (default), Light, Dark. The app sets the window's theme, so Mica's tint, the title bar and the page's `prefers-color-scheme` follow together. Check first whether Tauri's `Window::set_theme` also sets WebView2's preferred color scheme; if not, record what does.
- [ ] **Backdrop:** Mica (default), Mica Alt (`Effect::Tabbed`), Acrylic, None (solid). It changes live. On Windows 10 it is disabled, with the reason as its tooltip.
- [ ] **Accent color:** Amber only, until ticket 09 adds Windows' accent color.
- [ ] **Edit with:** "Default (name of the `.txt` program)", the chosen editor if any, and "Choose..." which opens v1 ticket 15's file dialog.
- [ ] Each change is saved in the settings file and sent to every open window.
- [ ] Page tests: opening and closing (Esc, outside, gear), the focus, and the change the page sends for each control.
- [ ] App tests: theme and backdrop kept across a restart, defaults when missing, and a broken value loads the default.
- [ ] Checked by hand: every theme with every backdrop, light and dark Windows, transparency effects off, high contrast.

## Comments

**2026-11-16, from the review of tickets 03 and 04.** The theme has to reach WebView2's preferred color scheme, not only CSS's `color-scheme`: github-markdown-css switches light and dark on `@media (prefers-color-scheme)`, while the page's own colors and the accent roles in `page/src/accent.css` use `light-dark()`, which follows `color-scheme`. Setting only `color-scheme` on `:root` would recolor the page's parts but leave GitHub's colors in the other theme.

**2026-11-16, from v1 ticket 15.** What the settings file and Edit leave for this ticket:

- The store is `SettingsStore` in `app/src/settings.rs`, managed state loaded in `setup` before any window opens. Add each value as a field of `Settings` (the struct has `#[serde(default)]`, camelCase), with an accessor like `editor()`, and change it with `update(|settings| ...)`, which saves the whole file and keeps a change it couldn't save for the session (`SettingsError::Write`).
- "A broken value loads the default" is not what the file does now: any value of the wrong type fails the whole file, which is kept as `settings.json.bad`, and every value takes its default. Giving each field its own fallback (for example a `deserialize_with` that turns an error into the default) is this ticket's; a file that isn't JSON at all should still go to `.bad`.
- Edit with: "Choose..." is the `chooseEditor` command (`page/src/edit.ts`, the app's `choose_editor`), which already refuses Downlight and anything but an `.exe`. "Default" needs a new command that clears the editor (`update(|settings| settings.editor = None)`). The text file program's file comes from `platform::text_file_program()`, which is None when the system names none, as for some packaged programs (Windows 11's Notepad may be one); a friendly name for "Default (name)" needs `AssocQueryStringW` with `ASSOCSTR_FRIENDLYAPPNAME` in the Windows module.
- When Choose editor... leaves the right-click menu, remove `"chooseEditor"` from `menu` in `commands.ts` and keep the command, which the flyout runs.

**2026-11-16, from the design review of v1 tickets 12 to 15.** Ticket 12 (Settings reach every page) is now a blocker: it replaces the per-field accessors suggested above with one snapshot (`SettingsStore::get`, `update` returning the new snapshot) and sends it to every window, and `backdrop.ts` already listens for changes there.
