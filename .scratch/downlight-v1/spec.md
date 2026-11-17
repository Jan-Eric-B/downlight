# Downlight v1

Status: ready-for-agent

## Problem Statement

I want one app that opens every Markdown file I double-click: fast, looking the way GitHub shows it, and out of the way while I edit the file in another program. My current viewer, WinDown, is built on a WinUI Markdown control that can't keep up. A 1 MB file freezes the window for minutes, many documents render differently from GitHub, and every fix adds another workaround around the control. Reading a document should take a double-click and under a second, and saving in my editor should show the change without my doing anything.

## Solution

Downlight is a small, new viewer for Markdown files that becomes the default app for them on Windows 11. Double-clicking a file opens it in its own window. It looks like GitHub (GitHub's stylesheet, light or dark following Windows), sitting on Windows' Mica backdrop. When the file changes on disk, Downlight reloads it at the same reading position. Ctrl+E opens the document in my editor. Find, links between documents, Back and Forward, zoom, ticks on task lists and a copy button on code blocks cover reading. There is no editing, no settings window and no toolbar: commands are on the keyboard and in a right-click menu.

Downlight renders Markdown in Rust (comrak) and shows the result in the system webview through Tauri 2. It runs as one process with a window per document (ADR 0001). v1 ships for Windows x64 only; macOS and Linux come later.

## User Stories

### Opening documents

1. As a reader, I want Downlight to open when I double-click a Markdown file in Explorer, so that reading a document takes one step.
2. As a reader, I want Downlight to open `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn` and `.mdwn` files, so that every common Markdown extension works.
3. As a reader, I want to run `downlight <file>` from a shell, so that I can show a document from the command line.
4. As a script author, I want `downlight <file>` to return at once, so that a script or agent that shows me a document doesn't wait for me to close it.
5. As a reader, I want Ctrl+O to show an open dialog filtered to Markdown files, so that I can open a document from inside Downlight.
6. As a reader, I want to drop a Markdown file onto a window to show it there, so that I can open files from anywhere I can drag them.
7. As a reader, I want a short message when I drop a file that isn't Markdown, so that I know why nothing opened.
8. As a reader, I want each document in its own window, so that I can place documents side by side with the usual window tools.
9. As a reader, I want opening a document that is already open to bring its window to the front, so that I don't end up with duplicates.
10. As a reader, I want the window title to show the file name and its folder (`README.md - docs - Downlight`), so that I can tell ten READMEs apart in Alt+Tab and on the taskbar.
11. As a reader, I want a window without a document to offer "Open file" and say that I can drop a file, so that starting Downlight from the Start menu isn't a dead end.
12. As a reader, I want a missing or unreadable file to show its error in the window, so that I see what went wrong instead of a blank page.
13. As a reader, I want Ctrl+W to close the window, so that closing a document is one key.

### Rendering like GitHub

14. As a reader, I want documents styled like GitHub, so that a README looks the way it does on GitHub.
15. As a reader, I want light or dark to follow Windows, so that Downlight matches the rest of my desktop without a setting.
16. As a reader, I want tables, task lists, strikethrough and autolinks, so that GitHub Flavored Markdown renders fully.
17. As a reader, I want footnotes rendered with links back and forth, so that annotated documents read as on GitHub.
18. As a reader, I want alerts (`> [!NOTE]`, `> [!WARNING]` and the others) shown as colored callouts, so that warnings stand out as they do on GitHub.
19. As a reader, I want headings to get the same anchors GitHub gives them, so that links like `#installation` written for GitHub work in Downlight.
20. As a reader, I want emoji shortcodes such as `:tada:` shown as emoji, so that documents written for GitHub read the same.
21. As a reader, I want YAML front matter shown as a code block at the top, so that I can see it without it breaking the document.
22. As a reader, I want code blocks with a language highlighted, so that code is readable.
23. As a reader, I want a copy button on each code block, so that I can copy a command without selecting it.
24. As a reader, I want to select text and copy it with Ctrl+C, so that I can quote a document.
25. As a reader, I want the raw HTML that GitHub allows (`<details>`, `<picture>`, `<img width>`, `align`, `<kbd>`, `<sub>`, `<sup>`) to render, so that READMEs with collapsible sections and themed logos look right.
26. As a reader, I want scripts, styles and event handlers in a document never to run, so that opening a downloaded file is safe.
27. As a reader, I want remote images and badges to load as on GitHub, so that READMEs look complete.
28. As a reader, I want images on network shares never to load, so that a document can't make Windows send my credentials to a server just because I opened it.
29. As a reader, I want relative image paths to resolve against the document's folder, so that images in a repository show.
30. As a reader, I want a file with a byte order mark read in that encoding (UTF-8, UTF-16 LE or BE), so that files written by Windows PowerShell 5.1 display correctly.
31. As a reader, I want a file without a byte order mark read as UTF-8 when it is valid UTF-8, and as Windows-1252 otherwise, so that older files with umlauts show umlauts.

### Speed

32. As a reader, I want the first screen of a 100 KB document visible within 1 second of my double-click (**first paint**), so that Downlight feels instant.
33. As a reader, I want a further window to show its document within 300 ms while Downlight is already running, so that opening several documents is quick.
34. As a reader, I want a 1 MB document never to stop the window from answering for more than 100 ms, and to be fully shown within 2 seconds, so that large files don't freeze anything.

### Reload

35. As a reader, I want the document to **reload** when its file is saved, so that I see my edits without doing anything.
36. As a reader, I want editors that save by writing a new file and renaming it over the old one to trigger a reload too, so that reload works with every editor.
37. As a reader, I want a reload to keep my reading position, so that I don't lose my place each time I save.
38. As a reader, I want F5 and Ctrl+R to reload by hand, so that I can force it.
39. As a reader, I want a bar saying the file was deleted or moved when it disappears, while the last render stays visible, so that I can keep reading and know the window is stale.
40. As a reader, I want the document to reload when its file comes back, so that a delete-and-recreate by a tool doesn't leave me stuck.

### Edit

41. As a reader, I want Ctrl+E to open the document in my **editor**, so that changing it is one key away.
42. As a reader, I want the editor to default to the program my system uses for plain text, so that Edit works with no setup.
43. As a reader, I want "Choose editor..." to let me pick any program as my editor, remembered from then on, so that Edit uses the editor I prefer.
44. As a reader, I want Edit never to open Downlight itself, so that being the default app for Markdown doesn't break Edit.

### Find

45. As a reader, I want Ctrl+F to open a find bar, so that I can **find** text in the document.
46. As a reader, I want matches highlighted as I type, with "3 of 12" shown, so that I see how many there are and where I am.
47. As a reader, I want Enter and F3 to go to the next match and Shift+Enter and Shift+F3 to the previous one, wrapping at the ends, so that I can step through matches.
48. As a reader, I want Find to ignore case, so that I find words wherever they start a sentence.
49. As a reader, I want Esc to close the find bar and clear the highlights, so that the document is clean again.
50. As a reader, I want Find to look only in the shown document, so that its results are about what I'm reading.

### Links and navigation

51. As a reader, I want a link to another Markdown file to show that document in the same window, so that I can read a set of documents like a website.
52. As a reader, I want a link to a heading to scroll to it, including a heading in another document, so that tables of contents work.
53. As a reader, I want Ctrl+click on a link to a Markdown file to open it in a new window, or bring its window to the front if it is open, so that I can keep both documents in view.
54. As a reader, I want `http`, `https` and `mailto` links to open in my browser or mail app, so that web links work as usual.
55. As a reader, I want a link to a local file that isn't Markdown to show the file selected in Explorer, never open or run it, so that a document can't start a program with one click.
56. As a reader, I want a link to a folder to open it in Explorer, so that links to folders are useful.
57. As a reader, I want links with any other scheme to do nothing, so that a document can't hand odd URLs to other programs.
58. As a reader, I want every link to show its target on hover, so that I know where a link goes before I click.
59. As a reader, I want a link to a missing file to show a message, so that a broken link doesn't look like a dead click.
60. As a reader, I want Back and Forward (Alt+Left, Alt+Right and the mouse's side buttons) to move between the documents and headings I visited in this window, so that following links is reversible.
61. As a reader, I want Back to return me to where I was reading, so that a detour costs nothing.
62. As a reader, I want each window's history to be its own and gone when the window closes, so that windows don't affect each other.

### Ticks

63. As a reader, I want to tick task list items, so that I can follow a checklist while I work through it.
64. As a reader, I want my **ticks** to survive a reload as long as the number of task items stays the same, so that saving the file doesn't wipe my progress.
65. As a reader, I want my ticks cleared when the number of task items changes, so that a tick never lands on the wrong item.
66. As a reader, I want my ticks to survive Back and Forward in the same window, so that following a link doesn't wipe them.
67. As a reader, I want ticks never written to the file and gone when the window closes, so that Downlight never changes my files.

### Zoom and window

68. As a reader, I want Ctrl+plus, Ctrl+minus, Ctrl+0 and Ctrl+wheel to zoom, so that I can read small text.
69. As a reader, I want the zoom level remembered and used by every new window, so that I set it once.
70. As a reader, I want a new window to open at the size of the last window I closed, offset from the active window, so that windows don't stack exactly on top of each other.
71. As a reader, I want the native title bar with Windows' snap layouts, so that the window behaves like every other window. (Replaced by [Window design](../window-design/spec.md): the page draws the title bar, and snap layouts still work, ADR 0005.)
72. As a reader, I want Mica behind the whole window on Windows 11, so that Downlight looks at home on my desktop.
73. As a reader on Windows 10, with transparency effects turned off, or in high contrast, I want a solid, readable background, so that the document is always legible.
74. As a reader, I want a right-click menu on the document with Back, Forward, Reload, Find, Edit, Choose editor..., Open, Zoom and Copy, so that every command is reachable with the mouse.
75. As a reader, I want every command to have a keyboard shortcut, so that I can work without the mouse.

### Installing

76. As a Windows user, I want a per-user installer that needs no admin rights, so that I can install Downlight on a work machine.
77. As a Windows user, I want Downlight to use the WebView2 runtime that Windows already has, so that nothing else needs installing.
78. As a Windows user, I want Downlight listed in "Open with" and in Default apps for Markdown files, so that I can make it the default.
79. As a Windows user, I want the installer to add Downlight to my PATH, so that `downlight <file>` works in any new shell.
80. As a Windows user, I want uninstalling to remove Downlight and its file associations, so that nothing is left behind.
81. As a Windows user, I want Downlight to make no network requests of its own, so that it is safe on a work network and works offline.

### Maintaining

82. As the maintainer, I want the stack's performance measured in a spike before any feature work, so that the choice of Tauri, comrak and the system webview rests on numbers.
83. As the maintainer, I want rendering chosen by file format behind one entry point, with Markdown the only format in v1, so that adding CSV or SVG later is one new module.
84. As the maintainer, I want code that differs per OS kept in one module per OS, so that the macOS and Linux ports touch only those modules.
85. As the maintainer, I want the rendering and link logic in a crate with no Tauri dependency, so that its tests are fast and don't need a window.

## Implementation Decisions

### Stack

- Tauri 2 on the latest stable 2.x (2.12.5 at the time of writing; Tauri 3 is in alpha and stays out of v1).
- comrak 0.59 for Markdown, with syntect 5 through comrak's highlighter plugin interface: Downlight's own adapter, bat's grammars from two-face, and the pure-Rust regex engine (fancy-regex). Scopes map to GitHub's own `pl-*` classes, which github-markdown-css colors for light and dark, so switching needs no new render (ADR 0004). PowerShell's grammar needs Oniguruma, so PowerShell shows as plain code.
- ammonia 4 for sanitizing, `encoding_rs` for decoding, `notify` for watching.
- The page is plain TypeScript built with Vite, with no UI framework and no CSS framework: github-markdown-css (MIT) in its light and dark variants, plus a small stylesheet of Downlight's own.
- One process, a window per document (ADR 0001). Rendering happens in Rust, not in the page; ADR 0002 records why once the spike confirms the numbers.

### Modules

**Core** (a crate with no Tauri dependency). It does the rendering and link work but reads and watches no document files, and its public interface is the main test seam:

- **Render**: takes a file's bytes and its path, and returns the rendered document: sanitized HTML in which every block carries its **line number** (the line of the file where it starts, from comrak's source positions). Rendering is chosen by file format; Markdown is the only format in v1.
  - Decoding: a byte order mark decides (UTF-8, UTF-16 LE, UTF-16 BE); otherwise valid UTF-8 is UTF-8; otherwise Windows-1252.
  - comrak options: GFM (tables, strikethrough, autolinks, task lists, tagfilter), footnotes, alerts, GitHub-style heading anchors, emoji shortcodes, front matter with `---` (shown as a YAML code block), raw HTML allowed, source positions on.
  - Sanitizing with a GitHub-like allowlist: keeps `<details>`, `<summary>`, `<picture>`, `<source>`, `<img>` with `width`, `height` and `align`, `<kbd>`, `<sub>` and `<sup>`, as well as the task list checkboxes, the heading ids, the highlighting classes and the line number attributes. It removes scripts, styles, event handlers and `javascript:` URLs.
  - Images: relative paths resolve against the document's folder and are served to the page through Downlight's own scheme. Remote `http(s)` images stay. Images on network shares (UNC paths, `file://server/`) are removed.
- **Link resolution**: takes the document's path and a link target, and returns one action:
  - show a document in this window, optionally at a heading;
  - scroll to a heading in this document;
  - reveal a file in the file manager;
  - open a folder;
  - hand a URL to the system (`http`, `https` and `mailto` only);
  - nothing.

  A file is Markdown when it has one of the six extensions. A path that leads to a network share, by its name or through a symbolic link, asks the reader first, naming the share's host, and is looked at only after a yes (ticket 20); with a yes it resolves like a local path. A share on the host whose share the document's own path names asks nothing, since reading the document connected there already.

**App** (the Tauri crate):

- **One process.** A second launch passes its paths to the running process and exits; the process exits when its last window closes. A document that is already open (paths compared after resolving them, ignoring case on Windows) has its window brought to the front instead of opening a second one.
- **Opening.** Explorer and the command line open a new window. Ctrl+O and dropping a file show the document in the current window and record it in Back history, as following a link does. A window started without a file shows the empty state. The app reads the document's file and passes its bytes to the core's Render.
- **Watching.** The app watches each shown document's file: given its path, it reports Changed, Gone and Back. It watches the file's folder rather than the file, so that saves by rename are seen, and reports a change 150 ms after the last event. On Changed and Back it reads the file again and passes its bytes to the core's Render; the page then shows the result as a reload.
- **Windows.**
  - Native title bar. The title is the file name, its parent folder, then "Downlight".
  - A new window takes the size of the last window that closed and is offset down and to the right of the active window.
  - Reading, rendering and watching run off the main thread, so the window keeps answering while a large document renders.
- **Mica.** On Windows 11 the window is transparent with the Mica window effect, and the page draws no background of its own, so the document sits on Mica. Code blocks, tables and quotes keep github-markdown-css's fills. Windows 10 and Linux get a solid background. macOS vibrancy comes with the macOS port.
- **Settings.** A small file in the app's config folder holds the chosen editor, the zoom level and the last window size. There is no settings window.
- **Edit.** Ctrl+E opens the document in the chosen editor if one is set and still exists. Otherwise it uses the program the system uses for `.txt` files, started through the shell's handler API so that the packaged Notepad on Windows 11 works. Otherwise it uses Notepad. "Choose editor..." opens a file dialog for a program and remembers the choice.
- **Per-OS module.** Editor launch, Mica, file association details and reveal-in-file-manager live in one module per OS. v1 implements Windows only.
- **Right-click menu.** A native popup menu with Back, Forward, Reload, Find, Edit, Choose editor..., Open, Zoom in, Zoom out, Reset zoom and Copy.

**Page** (TypeScript):

- **Document view.** Shows rendered HTML in one of three ways: as a new document, as a reload, or as a Back or Forward step.
  - Task list checkboxes are tickable.
  - On a reload, ticks are kept when the number of task items is the same and cleared otherwise.
  - Back and Forward restore the ticks and reading position of that history entry.
  - The reading position is the line number of the block at the top of the view; after a reload the view scrolls back to that block.
  - Link clicks go to the core's link resolution through an app command, and the page carries out the action. Hovering a link shows its target.
- **Find.** A plain, case-insensitive text match over the document's text. Matches are highlighted without changing the document's DOM (the CSS Custom Highlight API). The bar shows "N of M" and supports next and previous with wrapping, scrolling the current match into view. Esc closes the bar and clears the highlights.
- **Code copy button** on each code block.
- **Zoom** through the webview's zoom, with the level saved in the settings. (Changed by [Window design](../window-design/spec.md): zoom applies to the document view only, so the page's title bar keeps its size.)

### Keyboard shortcuts

| Keys | Command |
| --- | --- |
| Ctrl+O | Open |
| Ctrl+E | Edit |
| Ctrl+F | Find |
| Enter, F3 | Next match |
| Shift+Enter, Shift+F3 | Previous match |
| Esc | Close Find |
| F5, Ctrl+R | Reload |
| Alt+Left, mouse back button | Back |
| Alt+Right, mouse forward button | Forward |
| Ctrl+plus, Ctrl+minus, Ctrl+wheel | Zoom in, zoom out |
| Ctrl+0 | Reset zoom |
| Ctrl+C | Copy |
| Ctrl+W | Close window |

### Distribution

- Tauri's NSIS installer in per-user mode: no admin rights, file associations for the six extensions under the current user, and the install folder added to the user's PATH.
- The system WebView2 only (WebView2 install mode "skip"); the installer never bundles or downloads a runtime.
- Unsigned for v1. No MSI, no MSIX, no updater plugin, no network requests of Downlight's own.
- App identifier `io.github.jan-eric-b.downlight`, product name Downlight, command `downlight`.

### Content security

- The page's content security policy allows Downlight's own scripts and nothing else. Images may come from Downlight's own scheme for local files, from `data:` URLs and from remote `http(s)` addresses.

### Performance targets and the spike

The first ticket is a spike: Tauri plus comrak rendering a file given on the command line into github-markdown-css, with Mica on. It measures, from inside the app:

- the time from the process start to the page's first frame after the document is inserted (first paint);
- page tasks longer than 100 ms, from the page's long-task reports.

| Case | Target |
| --- | --- |
| Cold start, 100 KB document | First paint under 1.0 s |
| A further window in the running process | First paint under 300 ms |
| 1 MB document | The window never stops answering for more than 100 ms; fully shown within 2 s |
| Memory | Recorded, no ceiling |

If the cold start takes more than 1.5 s, the stack is reopened before any feature work. The spike also checks three things about Mica:

- that Mica shows in the native title bar;
- that turning off "Transparency effects" and switching on high contrast give a solid background;
- whether a transparent window flashes white while it opens.

## Testing Decisions

- A good test drives a public interface and checks what a user or caller can observe: the HTML, the link action, the watch events, the state of the page. It doesn't check how the result came about. comrak, ammonia and the webview are never mocked.
- **Core** (`cargo test`): the main seam.
  - Render tests feed Markdown in and check the HTML: decoding (byte order marks, UTF-8, Windows-1252), the GFM features, alerts, footnotes, heading anchors, emoji, the front matter block, the highlighting classes, line numbers on blocks, the sanitizing (what is removed and what is kept), remote images kept, images on network shares removed, and relative images resolved.
  - Link resolution is a table of document paths and targets with the action each one gives.
- **App watching** (`cargo test`): the app's watching takes a path and reports events, so its tests need no window. They use real temporary files: a rewrite in place, a rename over the file, a delete, a recreate. There are only a few of these tests, since each waits about 150 ms.
- **Page** (Vitest with happy-dom):
  - Document view: showing a document, a reload with the same number of task items (ticks kept), a reload with a different number (ticks cleared), Back and Forward (ticks restored), and which block a reload anchors to.
  - Find: the count, the current match, next and previous, wrapping, Esc.
  - The page tests use a small set of rendered fixtures. The core's tests produce and check them, so both sides agree on the HTML's shape.
  - happy-dom does no layout, so scroll offsets are not tested here.
- **Manual checklist**, for what needs a real window:
  - one process, and an open document's window coming to the front;
  - `downlight <file>` returning at once;
  - Edit with the `.txt` program, the packaged Notepad, and a chosen editor;
  - zoom shared across windows; window size and offset;
  - Mica, transparency effects off, high contrast, Windows 10;
  - the right-click menu and every shortcut;
  - reload during real saves from VS Code and Notepad++;
  - the reading position after a reload, and Back;
  - the installer, PATH, the file associations, "Open with", Default apps, and uninstalling;
  - first paint and the 1 MB document.
- **Prior art**: none. The repository has no code yet, so these tests set the pattern.

## Out of Scope

**v1.x**, the next releases:

- edit at the reading line;
- full width, Keep on top, full screen (full width and Keep on top moved into v1 with [Window design](../window-design/spec.md));
- pasting or dropping Markdown text;
- Copy as Markdown;
- Outline;
- printing and PDF through the webview's print dialog;
- the last 5 documents listed in the empty window, with no automatic reopening.

**Later**, not planned yet:

- tabs;
- Mermaid and math, loaded only for documents that contain them;
- Find options (case, whole word, regex);
- a table of contents sidebar, after Outline;
- the Explorer preview pane;
- line numbers in the margin, switched with a shortcut, off by default;
- the overview ruler, after Outline;
- other formats: CSV/TSV as a table and SVG as an image first, then JSON and XML as collapsible trees. Downlight shows up in "Open with" for them but never asks to be their default app;
- a Word, PDF and PowerPoint converter, as a separate optional component;
- reader settings;
- writing ticks to the file, opt-in;
- opening non-Markdown local files with their default app, opt-in;
- the macOS port with vibrancy, and the Linux port;
- before publishing: a setting to block remote images, and a review of how discoverable the commands are.

**Out:**

- editing in the app;
- search across files;
- reading from stdin or a `--text` argument;
- opening URLs;
- TOML front matter;
- a theme override and an accent color (reversed 2026-11-13: [Window design](../window-design/spec.md) adds both, with a title bar, its buttons and a settings flyout);
- WinDown's code, rules and backlog. Downlight is a new app and ports nothing.

**Platforms:** v1 is Windows x64 only.

## Further Notes

- **Defaults set without an explicit decision**: the app identifier and command name; dropping a non-Markdown file shows a message; a missing or unreadable file shows its error in the window; Ctrl+O and dropping a file show the document in the current window with Back history; no crash log and no telemetry in v1.
- **Versions** were taken from the crates.io registry on 2026-11-03, since Context7 was not available: comrak 0.59.0, tauri 2.12.5, ammonia 4.2.4, window-vibrancy 0.8.1. Check them again when adding each dependency.
- **The work machine** has WebView2 156 installed system-wide, no AppLocker rules, and user-mode code integrity off, so an unsigned per-user build runs.
- **MDhero** (MIT, a Tauri Markdown viewer and editor) is a reference to read, not a base. Small pieces, such as its `notify` watcher, its content security policy and its file association setup, may be copied with its MIT notice.
- On Linux, the system webview (WebKitGTK) is a package dependency, so "nothing else to install" holds only on Windows and macOS.
- **Terms** follow the glossary: Document, Editor, Find, Tick, Reload, First paint, Line number, Overview ruler.
