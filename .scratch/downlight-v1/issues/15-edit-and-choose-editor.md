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
