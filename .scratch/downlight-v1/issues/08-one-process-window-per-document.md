# 08: One process, a window per document

**What to build:** Downlight runs as one process, and each document opened from Explorer or the command line gets its own window in it (ADR 0001). A second launch passes its file to the running process and exits, so `downlight <file>` returns at once. A document that is already open brings its window to the front instead of opening twice.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 02 (Performance spike and stack decision)

**Status:** ready-for-agent

- [ ] A second launch with a file passes it to the running process and exits at once, so `downlight <file>` in a shell returns immediately.
- [ ] Each document opened from the command line or Explorer gets its own window in the one process.
- [ ] Opening a document that is already open (paths compared after resolving them, ignoring case on Windows) brings its window to the front and opens no second window.
- [ ] The window title is the file name, its parent folder, then "Downlight", for example `README.md - docs - Downlight`.
- [ ] The process exits when its last window closes; there is no tray icon.
- [ ] A missing or unreadable file shows its error in the window.
- [ ] Ctrl+W closes the window.
