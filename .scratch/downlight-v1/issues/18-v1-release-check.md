# 18: v1 release check

**What to build:** Confirm that v1 is done: the performance targets hold with every feature on, the manual checklist passes, and Downlight replaces WinDown as the default app for Markdown on the work machine.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 05 (Files in any common encoding), 07 (Syntax highlighting and copy buttons), 13 (Ticks), 14 (Find), 16 (Zoom and window size), 17 (Installer and file associations), 19 (Startup errors are shown), 20 (No credentials sent for remote images), 30 (A document's watch never goes quiet for good), 31 (Large and pathological documents fail gracefully), 32 (Local images show their file's latest version after a reload), 34 (Small decisions left from the architecture review), 35 (Edit at the reading position), 36 (Third-party notices ship with the app), 37 (A window whose webview fails to start closes)

**Status:** closed

- [x] Measured with the timing switch on the installed build, with the results written under a "Results" heading in this ticket: cold first paint for 100 KB under 1.0 s, a further window under 300 ms, the 1 MB documents never stopping the window for more than 100 ms and fully shown within 2 s, and memory recorded.
- [x] The spec's manual checklist passes:
  - [x] one process, and an open document's window coming to the front;
  - [x] `downlight <file>` returning at once;
  - [x] Edit with the `.txt` program, the packaged Notepad and a chosen editor;
  - [x] zoom shared across windows, and the window size and offset;
  - [x] Mica, transparency effects off, high contrast and Windows 10 (if one is available);
  - [x] the right-click menu and every keyboard shortcut;
  - [x] reload during real saves from VS Code and Notepad++;
  - [x] the reading position after a reload and after Back;
  - [x] the installer, the PATH, the file associations, "Open with", Default apps and uninstalling.
- [x] Downlight is installed and set as the default app for Markdown on the work machine.
- [x] WinDown is uninstalled.

## Results

**2026-11-19, measured** with `npm run measure` on the release build of commit 03b5e1d plus the settings flyout's focus ring fix (CSS only), `target\release\downlight.exe`, Windows 11 Enterprise build 26200, with nothing else of the agent's running. The installed build is the same code without that CSS line. Medians of 5 runs, in ms. Memory is the sum over Downlight and its WebView2 processes (7), private bytes (working set in brackets).

| Target | Result |
| --- | --- |
| Cold first paint, 100 KB, under 1.0 s | **met**: 807 (shown at 823) |
| A further window, under 300 ms | **met**: 276 from the window opening, 403 from the second launch, which returns in 115 |
| 1 MB documents fully shown within 2 s | **met**: mixed 1198, prose 848 |
| 1 MB documents never stop the window for more than 100 ms | **met on the medians**, every one 0 once the page has the document; worst single runs 105 (1 MB mixed, after first paint) and 107 (typing into Find on 1 MB mixed) |
| Memory recorded | 100 KB: 297 MB (519); 1 MB mixed: 339 (565); 1 MB prose: 225 (450); two windows of 100 KB: 393 (655) |

| Measure (median of 5) | 100 KB | 1 MB mixed | 1 MB prose |
| --- | ---: | ---: | ---: |
| First paint | 807 | 1198 | 848 |
| Shown | 823 | 1211 | 865 |
| The page's navigation starts | 605 | 687 | 578 |
| The page receives the HTML | 766 | 1078 | 775 |
| Read and render | 499 | 978 | 49 |
| Longest task before the page has the document | 0 (worst 135) | 121 (worst 194) | 0 (worst 0) |
| Longest task or frame after | 0 (worst 0) | 0 (worst 105) | 0 (worst 0) |

- **Before the document:** the long task before the page has the document is the one ticket 02 found: loading the page itself, while the window is still hidden (it shows at first paint, ticket 04), so the reader never sees it.
- **Scrolling** through both 1 MB documents: no task or frame over 100 ms in any run.
- **Reload** of 1 MB mixed after a save in the middle: shown 1095 ms after the save (read 343, render 481, insert 63, anchor 70), no task over 100 ms, and the reading position moved 0 lines.
- **Find**, typing "e", "the", "lamplight", "qxzjv": 14 to 16 ms per keystroke to its count, 143 to 178 ms for the first, 39 to 40 for the slowest later one.
- Against ticket 31's loaded session, the 100 KB first paint went from 1040 to 807 ms, with WebView2 starting the page at 605 ms instead of 837.

## Comments

**2026-11-13, from the architecture review.** Hand checks found by the review, for the checklist: renaming or deleting an open document's folder in Explorer (ticket 30); a watched document on a network share after sleep or a VPN reconnect (ticket 30); a local image changed, then the document saved (ticket 32); Ctrl+P, Ctrl+J, F7 and Esc doing nothing of WebView2's own (ticket 29); a 20 MB document and a huge image (ticket 31); a heading scrolled to the top while "File deleted or moved" shows.

**2026-11-19, the manual checklist passed.** The repo owner ran the checklist on the release build and installer of commit 03b5e1d, with the review's hand checks above and those of tickets 30 to 37: one process, `downlight <file>`, Edit, zoom and window size, the backdrops with transparency effects off and high contrast, the right-click menu and every shortcut (WebView2's own keys doing nothing), reload during real saves, the reading position, and the installer, PATH, associations, Open with, Default apps and uninstalling. Downlight is the default app for Markdown and WinDown is uninstalled. No Windows 10 result was reported. Every blocker but the measurement is closed.

Left: the measurement (the first box), on an idle machine, which also takes ticket 31's 100 KB first paint again. Found during the checks, not blockers of this ticket unless triaged so: ticket 38 (a large document's load can't be left) and window design 15 (the backdrops against Windows' own).

**2026-11-19, accepted by the repo owner.** The measurement above is accepted as meeting the targets: the two single runs over 100 ms (105 and 107) are within noise, with every median at 0, and the build measured differs from the installed one only by a CSS rule. Every box is ticked and v1's release check is closed.
