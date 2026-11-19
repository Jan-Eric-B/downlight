# 18: v1 release check

**What to build:** Confirm that v1 is done: the performance targets hold with every feature on, the manual checklist passes, and Downlight replaces WinDown as the default app for Markdown on the work machine.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 05 (Files in any common encoding), 07 (Syntax highlighting and copy buttons), 13 (Ticks), 14 (Find), 16 (Zoom and window size), 17 (Installer and file associations), 19 (Startup errors are shown), 20 (No credentials sent for remote images), 30 (A document's watch never goes quiet for good), 31 (Large and pathological documents fail gracefully), 32 (Local images show their file's latest version after a reload), 36 (Third-party notices ship with the app), 37 (A window whose webview fails to start closes)

**Status:** ready-for-human

- [ ] Measured with the timing switch on the installed build, with the results written under a "Results" heading in this ticket: cold first paint for 100 KB under 1.0 s, a further window under 300 ms, the 1 MB documents never stopping the window for more than 100 ms and fully shown within 2 s, and memory recorded.
- [ ] The spec's manual checklist passes:
  - [ ] one process, and an open document's window coming to the front;
  - [ ] `downlight <file>` returning at once;
  - [ ] Edit with the `.txt` program, the packaged Notepad and a chosen editor;
  - [ ] zoom shared across windows, and the window size and offset;
  - [ ] Mica, transparency effects off, high contrast and Windows 10 (if one is available);
  - [ ] the right-click menu and every keyboard shortcut;
  - [ ] reload during real saves from VS Code and Notepad++;
  - [ ] the reading position after a reload and after Back;
  - [ ] the installer, the PATH, the file associations, "Open with", Default apps and uninstalling.
- [ ] Downlight is installed and set as the default app for Markdown on the work machine.
- [ ] WinDown is uninstalled.

## Comments

**2026-11-13, from the architecture review.** Hand checks found by the review, for the checklist: renaming or deleting an open document's folder in Explorer (ticket 30); a watched document on a network share after sleep or a VPN reconnect (ticket 30); a local image changed, then the document saved (ticket 32); Ctrl+P, Ctrl+J, F7 and Esc doing nothing of WebView2's own (ticket 29); a 20 MB document and a huge image (ticket 31); a heading scrolled to the top while "File deleted or moved" shows.
