# 17: Installer and file associations

**What to build:** A per-user installer that needs no admin rights puts Downlight on a work machine, registers it for the six Markdown extensions so that it appears in "Open with" and in Default apps, and adds it to the PATH. It uses the system WebView2 only and removes everything again on uninstall.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 08 (One process, a window per document)

**Status:** ready-for-agent

- [ ] Tauri's NSIS installer in per-user mode installs without admin rights.
- [ ] `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn` and `.mdwn` are associated for the current user, and Downlight appears in "Open with" and in Default apps for them.
- [ ] The install folder is added to the user's PATH, so `downlight <file>` works in a new shell.
- [ ] WebView2 install mode is "skip": no runtime is bundled or downloaded.
- [ ] The identifier is `io.github.jan-eric-b.downlight` and the product name is Downlight.
- [ ] The installer is unsigned, with no updater and no network requests of Downlight's own.
- [ ] Uninstalling removes the app, its file associations and its PATH entry.
- [ ] With Downlight set as the default app, double-clicking a `.md` file in Explorer opens it in Downlight, and a second double-click opens a second window in the same process.

## Comments

**2026-11-10, from ticket 07.** The exe now embeds bat's grammars (two-face). Their licenses (`two_face::acknowledgement::listing()`) still need listing, checking against GPL-3.0 and shipping with the other notices.
