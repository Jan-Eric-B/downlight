# 08: The settings flyout

**What to build:** The Settings button (and Ctrl+,) opens a flyout under it with Theme, Backdrop, Accent color and Edit with. A change applies at once to every open window and is kept in the settings. "Choose editor..." leaves the right-click menu, and Settings joins it, last.

Spec: [Window design](../spec.md)

**Blocked by:** 05 (Open and Edit buttons, and the separators), v1 [15](../../downlight-v1/issues/15-edit-and-choose-editor.md) (Edit and Choose editor)

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
