# 08: One process, a window per document

**What to build:** Downlight runs as one process, and each document opened from Explorer or the command line gets its own window in it (ADR 0001). A second launch passes its file to the running process and exits, so `downlight <file>` returns at once. A document that is already open brings its window to the front instead of opening twice.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 02 (Performance spike and stack decision)

**Status:** closed

- [x] A second launch with a file passes it to the running process and exits at once, so `downlight <file>` in a shell returns immediately.
- [x] Each document opened from the command line or Explorer gets its own window in the one process.
- [x] Opening a document that is already open (paths compared after resolving them, ignoring case on Windows) brings its window to the front and opens no second window.
- [x] The window title is the file name, its parent folder, then "Downlight", for example `README.md - docs - Downlight`.
- [x] The process exits when its last window closes; there is no tray icon.
- [x] A missing or unreadable file shows its error in the window.
- [x] Ctrl+W closes the window.

## Comments

**2026-11-03, review.** The walking skeleton keeps one process-wide `CommandLineDocument` in Tauri's managed state and opens one window with the fixed label `"document"` (`app/src/main.rs`, `app/src/document.rs`). This ticket keys the document state and the window label per window, and moves the command line and window code out of `app/src/document.rs` into a module each.

**2026-11-03, implementation.** `tauri-plugin-single-instance` 2.5.5 hands a second launch's arguments and folder to the running process (3.0 is in alpha). `app/src/command_line.rs` turns a launch's arguments into absolute paths against the launching folder; `app/src/window.rs` keeps which window shows which document (`OpenWindows`), opens a window or brings the open one to the front, sets the title and closes the window for Ctrl+W; `app/src/document.rs` keeps only the render command, which reads the calling window's document. The per-OS module gives the key two paths to one document share (`document_key`: resolved, upper-cased per character as NTFS compares names). `npm run check` passes; the app crate has tests for the command line, the title and `OpenWindows`.

- The second launch waits in `SendMessageTimeoutW` until the plugin's callback returns, so the callback only hands the paths to a blocking task, which opens the windows.
- Checked by hand on the release build: one process; two windows titled `a.md - manual - Downlight` and `b.md - manual - Downlight`; `A.MD`, `sub\..\a.md` and a full path bring the open window to the front, also from minimized; `..\a.md` from a subfolder names the right folder; a missing file shows "Can't read ...: The system cannot find the file specified." in its window; a launch without a file opens a window titled Downlight; Ctrl+W closes the window and the process exits with the last one.
- `npm run measure` now opens the second window with a second launch on another 100 KB document (`mixed-100kb-other.md`), as a reader does. It reports how long that launch takes to return and the second window's first paint counted from the launch, which includes the hand-off that ticket 02's 214 ms left out. Two sessions today, at about 45 % CPU load (the 100 KB cold start was 904 and 970 ms, against 771 ms in ticket 02's tables):

  | Measure, median of 5 | Session 1 | Session 2 |
  | --- | ---: | ---: |
  | The second launch returns | 74 ms | 117 ms |
  | Second window's first paint, from the launch | not measured yet | 574 ms (380 to 610) |
  | Second window's first paint, from the window opening | 250 ms (231 to 412) | 460 ms (318 to 510) |

  The second launch returns as fast as Git Bash starts `cmd /c exit` (200 to 260 ms from the shell). The spec's 300 ms target for a further window is not met under this load, and the hand-off adds about 100 ms on top of opening the window. It needs a measurement on a quiet machine before deciding whether to work on it.
- Open: an error while opening a window for a second launch goes to stderr, which a release build doesn't show; ticket 19 shows it. A launch that arrives while the last window is closing can be lost, since the process exits under it; not seen, not guarded.

**2026-11-03, review fixes.** `document_key` moved from the per-OS module into `app/src/window.rs`; the per-OS module keeps only the case rule (`fold_case`, with tests), and the non-Windows copy of the path resolution is gone. Both launches go through `window::open_launch`, which also clears a closed window's entry (`forget_when_destroyed`). A window that fails to open doesn't keep the launch's other documents from opening; `open_launch` returns every failure as a `LaunchFailure` whose message names the document, and both launches print them on stderr until ticket 19 shows them. If the first launch ends up with no window, `setup` fails with those messages and the process exits instead of running with no window. The fallbacks for relative paths in `document_key` and `title_of` are gone, since the command line makes every path absolute. Checked by hand on the release build: one process, a second window for a second launch (the launch returned after 169 ms), and `A.MD` opens no third window.

**2026-11-09, closed.** Closed, with every criterion done. The 300 ms target for a further window, not met under load here, was met in ticket 04 (203 ms). Still open from the Comments: a launch that arrives while the last window is closing can be lost.
