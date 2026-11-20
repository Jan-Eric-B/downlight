# mdhero and Downlight: features compared

Written 2026-11-20 from a read-only pass over `C:\Users\debednaj\Source\Repos\mdhero` (version 0.2.10, last commit 2026-10-19) and this repo. Nothing in mdhero was built, installed or run, so every mdhero entry comes from its README, configuration and source. Paths in section 1 are relative to the mdhero repo root, and in section 2 to this repo.

## 1. mdhero

### What it is

A Markdown viewer and lightweight editor ("A beautiful, native Markdown viewer and lightweight editor for macOS, Windows and Linux", `README.md`). MIT licensed, one main author with outside contributors, 201 commits from 2026-04-22 to 2026-10-19.

**Tech stack** (`package.json`, `src-tauri/Cargo.toml`, `README.md` Stack):

- Tauri 2. The Rust side is thin: file read and write, a `notify` 7 watcher with `notify-debouncer-mini`, the native menus and the right-click menu, and the plugins `opener`, `dialog`, `cli`, `process`, `window-state`, `updater` and `single-instance` (Windows and Linux).
- SvelteKit with Svelte 5 as a static SPA, TypeScript, Tailwind CSS 4 with `@tailwindcss/typography`, Lucide icons.
- The page does the rendering: markdown-it 14 with `markdown-it-task-lists`, `markdown-it-anchor` and `markdown-it-texmath`, highlight.js 11 (core build with 25 languages registered), KaTeX, Mermaid 11, DOMPurify pinned to 3.3.x, and mark.js for Find.
- Tests: Vitest unit tests (`tests/unit/`, 36 files) and a few `cargo test` modules. pnpm.

**Platforms** (`README.md` Install, `.github/workflows/build.yml`): macOS on Apple Silicon (DMG, signed and notarized in CI, with a Quick Look extension), Windows x64 and ARM64 (MSI and NSIS), Linux x64 and arm64 (deb, rpm, AppImage). Android and iOS icons sit in `src-tauri/icons/`, but nothing builds for mobile.

**Shape:** one window per process, holding tabs (`tauri.conf.json` has a single `main` window). By default each launch from the file manager starts its own process and window, unless "Open files in the existing window" is on (`src-tauri/src/single_window.rs`).

### Features by area

Marks: **[partly]** means half built or limited, and **[planned]** means listed but not built.

#### Viewing and rendering

- **Markdown flavor:** markdown-it's default preset (CommonMark plus tables and strikethrough), with `linkify` (bare URLs become links) and `typographer` (smart quotes and dashes). `src/lib/renderer/pipeline.ts` (`initRenderer`).
- **Raw HTML is off** (`html: false`): HTML in a document shows as literal text. The one exception is a paragraph that is exactly `<div dir="rtl">`, `<div dir="ltr">` or `</div>`, which sets the direction of the blocks inside it. `pipeline.ts` (`addDirPlugin`).
- **Right-to-left text:** every text block gets `dir="auto"`, so Hebrew or Arabic paragraphs align themselves, and the wrapper divs above force a direction. `pipeline.ts`, `tests/fixtures/rtl-test/`.
- **Task lists** render as checkboxes, but they are disabled and can't be ticked (`taskLists, { enabled: false }`). `pipeline.ts`.
- **Heading anchors:** ids from `markdown-it-anchor` with a simple slug (lowercase, spaces to `-`, anything outside ASCII `\w` dropped, so non-ASCII letters are lost, unlike GitHub's slugs). `pipeline.ts`.
- **Math:** KaTeX through `markdown-it-texmath`, with `$...$` and `$$...$$`. `pipeline.ts`.
- **Mermaid diagrams:** `mermaid` code blocks become SVG after the HTML is inserted, with `securityLevel: "strict"`, labels as SVG text, the SVG sanitized again, and its own dark theme colors. If a diagram fails, its code block stays. `src/lib/components/MarkdownRenderer.svelte` (`renderMermaidBlocks`), `src/lib/renderer/mermaid-sanitize.ts`.
- **Syntax highlighting:** highlight.js with 25 languages and a few aliases (jsx, tsx, html, toml, text). A fence with an unknown language or none gets `highlightAuto`, which guesses the language. `pipeline.ts`.
- **Copy button** on each code block, showing "Copied" for 1.5 s. `MarkdownRenderer.svelte` (`addCodeCopyButtons`).
- **Wrap long code lines** instead of scrolling them (Scroll / Wrap switch). `src/lib/components/ReaderControls.svelte`, `wrapCodeBlocks` in `src/lib/stores/settings.ts`.
- **Front matter:** a leading `---` block is parsed by a naive line splitter into key/value pairs (lists in `[...]` split on commas), and up to 6 of them show as a bar above the document. `pipeline.ts` (`renderFull`), `src/lib/components/FrontmatterBar.svelte`.
- **Footnotes, GitHub alerts and emoji shortcodes:** not supported; no plugin for them is installed. `package.json`.
- **Local images:** relative paths resolve against the document's folder and load through Tauri's asset protocol. The Rust side serves only files inside the document's folder tree (widened to its git checkout, never above the home folder) or a pinned folder, after canonicalizing them. `pipeline.ts` (`resolveRelativeImages`), `src-tauri/src/commands.rs` (`allow_assets`, `asset_roots`).
- **Remote images** load: the content security policy allows `https:` in `img-src`. `src-tauri/tauri.conf.json`.
- **Image lightbox:** a click on an image shows it large; arrow keys step to the previous or next image, Esc closes. `src/lib/components/ImageLightbox.svelte`.
- **Link target on hover:** a tooltip shows a link's `href`. `MarkdownRenderer.svelte` (`addLinkTooltips`).
- **Encodings:** UTF-8 only. A byte order mark is dropped; a file that isn't UTF-8 (UTF-16 from Notepad, for example) is refused with a message to save it again as UTF-8. `commands.rs` (`read_markdown_file`), `pipeline.ts` (`stripBom`), `tests/unit/utf8-bom.test.ts`.
- **Which files open:** only the extensions md, markdown, mdown, mkd, mdx, txt and text can be read or written; anything else is refused before the disk is touched. `commands.rs` (`ALLOWED_TEXT_EXTENSIONS`).
- **Raw source view** (Cmd/Ctrl+U): the Markdown source in a monospace view, with line numbers when the setting is on. `src/routes/+page.svelte` (`rawMode`), `src/lib/utils/line-gutter.ts`.
- **Marp slides [partly]:** a document whose front matter says `marp: true` can be shown as a slideshow, split on top-level `---` (not inside code fences). Arrow keys, PageUp/PageDown, Space, Home and End move, Esc leaves, `paginate: true` shows page numbers, and a setting starts the slideshow by itself. It is "minimal" by its own description: no Marp themes or other directives. `src/lib/components/PresentationView.svelte`, `src/lib/renderer/slides.ts`, `marp-demo.md`.
- **Print styles:** white background, wrapped code, no page breaks inside tables, code and images. `src/app.css` (`@media print`).

#### Navigation

- **Tabs:** a Home tab plus one tab per document; drag to reorder, Cmd/Ctrl+1 to 9, Ctrl+Tab and Ctrl+Shift+Tab, Ctrl+PageUp/PageDown, Cmd+Shift+[ and ] on macOS, middle-click to close. Opening a file that is open switches to its tab. `src/lib/components/TabBar.svelte`, `+page.svelte` (`handleKeydown`, `cycleTab`), `src/lib/stores/tabs.ts`.
- **Tabs on the side:** a resizable panel on the left instead of the row on top; a tab shows its folder when another open tab has the same file name. `TabBar.svelte`, `tabsPosition` and `tabsWidth` in `settings.ts`.
- **Tab context menu:** Copy file name, Copy path, Open file location (reveals it in the file manager), Close tab, and moving the tabs between top and side. `TabBar.svelte`.
- **Table of contents sidebar:** the document's headings, the heading in view highlighted as you scroll, a click scrolls to it, and its width can be dragged. `src/lib/components/TableOfContents.svelte`, `src/lib/stores/toc.ts`.
- **Links:**
  - a `#heading` link scrolls in the page (the browser's own behavior);
  - a link to a local `.md` or `.markdown` file opens it in a new tab, or switches to its tab;
  - a `#fragment` on a link to another file is dropped ("cross-file anchor scroll is out of scope for now");
  - a link to another local file opens it in the system's default app, but the capability allows only PDF, txt, common image types and mp4, and a denylist refuses executables and scripts;
  - a link to a missing file shows a toast;
  - `http`, `mailto` and every other scheme go to the system's opener.
  
  `+page.svelte` (`handleLocalLink`, `EXECUTABLE_EXTENSIONS`), `src-tauri/capabilities/default.json`, `MarkdownRenderer.svelte` (`addLinkHandlers`).
- **No Back and Forward:** no history of visited documents or headings exists in the code.
- **Vim keys [partly]:** j and k scroll smoothly while held, d and u move half a page, gg and G go to the top and bottom, [ and ] go to the previous and next heading, / opens Find. `n` has a placeholder ("handled by SearchOverlay") but the find bar handles only Enter, so it does nothing. `+page.svelte` (`handleKeydown`), `src/lib/components/SearchOverlay.svelte`.
- **Reading progress bar:** a 3 px bar at the top shows how far down the document is scrolled. `src/lib/components/ProgressBar.svelte`.
- **Scroll to top** button. `src/lib/components/ScrollToTop.svelte`.
- **Reading position remembered per file:** the source line at the top of the view is saved (debounced, for up to 500 files) and restored with a smooth scroll when the file opens again, also after a restart. `src/lib/stores/readingProgress.ts`, `+page.svelte` (`restoreProgress`).
- **Scroll offset per tab**, restored on switching back to it. `+page.svelte`.

#### Search

- **Find** (Cmd/Ctrl+F, or /): literal, case-insensitive, all matches highlighted with mark.js (which wraps them in `<mark>` elements in the DOM), a current match scrolled to the center, a match count, Enter and Shift+Enter for next and previous with wrapping, Esc closes. It works in the rendered view, the raw view and the editor (through a highlight layer behind the text area). `SearchOverlay.svelte`, `src/lib/utils/text-search.ts`, `src/lib/components/Editor.svelte`.
- No Find options (case, whole word, regular expression) and no search across files.

#### Editing

- **Edit mode** (Cmd/Ctrl+E): the source in a text area, with line numbers and Markdown syntax coloring (a bounded line scanner, switchable in Settings), and Tab handled in the text area. `Editor.svelte`, `src/lib/utils/markdown-highlight.ts`.
- **Split mode:** the editor on the left and a live preview on the right, re-rendered 120 ms after typing stops, with the two panes' scrolling kept in step. `+page.svelte` (`splitMode`), `src/lib/utils/split-sync.ts`.
- **View / Split / Edit** switch in the toolbar; switching keeps the same source line in view. `src/lib/components/Toolbar.svelte`, `src/lib/utils/scroll-sync.ts`.
- **Save** (Cmd/Ctrl+S) writes the file; the watcher ignores the change for 1.5 s so the app's own save doesn't reload. `+page.svelte` (`handleSave`), `src/lib/tauri/watcher.ts`.
- **New document** (Cmd/Ctrl+T or the home screen): an untitled tab in the editor, with a Save As dialog on its first save. The README's shortcut table still calls Cmd+T "New tab (home)". `src/lib/tauri/files.ts` (`newDocument`, `saveAsNewDocument`).
- **Unsaved changes:** a dot in the tab and the toolbar, edits kept per tab, and a Save / Discard / Cancel prompt on closing a tab or quitting. `TabBar.svelte`, `tabs.ts`, `src/lib/utils/unsaved-prompt.ts`.
- **Changed on disk while edited:** the tab is marked, and Save asks before overwriting. `tabs.ts` (`diskChanged`), `+page.svelte`.
- An **empty file** opens straight in the editor. `files.ts` (`openFile`).

#### File handling

- **Open** (Cmd/Ctrl+O): mdhero's own dialog with Browse (the system dialog, several files at once, filtered to md, markdown, mdown, mkd and txt), a URL field, and Recent, Folders and Plans tabs. `src/lib/components/OpenDialog.svelte`, `files.ts` (`openFileDialog`).
- **Drag and drop:** the first dropped file opens when its extension is md, markdown, mdown, mkd or txt; any other drop is ignored without a message. `src/lib/components/DropZone.svelte`.
- **Paste Markdown** (Cmd/Ctrl+Shift+V): pasted text becomes a tab. An "LLM mode" (detected by itself) unescapes AI output that arrives as a JSON string with `\n` sequences. `src/lib/components/PasteModal.svelte`, `src/lib/utils/llm.ts`.
- **Open URL:** fetches a URL and shows it as a read-only tab, turning GitHub blob, Gist, GitLab and Bitbucket links into their raw URLs and refusing a page that returns HTML. `PasteModal.svelte`, `src/lib/utils/url.ts`. The content security policy's `connect-src` lists only GitHub's hosts, so GitLab, Bitbucket and "any public URL" may be blocked in the built app; I couldn't check this without running it.
- **Recent files:** the last 15, on the home screen and in the Open dialog, with Clear. `src/lib/stores/recents.ts`, `src/lib/components/EmptyState.svelte`.
- **Pinned folders:** lists up to 50 Markdown files per folder, newest first, down to 3 levels, skipping hidden folders, `node_modules`, `target` and the like. A pinned folder is also allowed as a source of local images. `src/lib/stores/pinned.ts`, `commands.rs` (`list_folder_md_files`).
- **Claude Code plans:** lists `~/.claude/plans/*.md`, newest first, as a section that can be hidden. `commands.rs` (`list_claude_plans`), `EmptyState.svelte`.
- **Reload:** each open tab's file is watched (its folder, so saves by rename count), events are debounced, and the tab's content is replaced; only the active tab repaints. Nothing anchors the view to a line on reload, so the scroll offset is simply left as it was. `src-tauri/src/watcher.rs`, `src/lib/tauri/watcher.ts`, `files.ts` (`reloadFile`).
- No reload by hand, and no notice when the file is deleted or moved: a failed reload only writes to the console. `files.ts` (`reloadFile`).
- **Session restore:** the last run's files reopen in the same order, with the same active tab (setting, on by default). `src/lib/tauri/session.ts`, `restoreTabsOnLaunch` in `settings.ts`.
- **Close on Escape** (setting, on by default): Esc closes the active tab, and quits the app when it was the last one. `+page.svelte` (`closeActiveTabOnEscape`).
- **[planned] Folder watch** (pinned folders' lists refresh by themselves). `README.md` Roadmap.

#### Export and sharing

- **Print and PDF:** the system print dialog from File > Print, Cmd/Ctrl+P or the toolbar. `+page.svelte` (`printDocument`), `src-tauri/src/menu.rs`.
- **Copy as rich text** (the rendered HTML, for Google Docs or Notion) and **Copy as Markdown** (the source), from the toolbar. `Toolbar.svelte`, `src/lib/utils/clipboard.ts`.
- **[planned] Export to self-contained HTML.** `README.md` Roadmap.
- **[planned] Compare / diff view.** `README.md` Roadmap.

#### Themes and look

- **Theme:** System, Light or Dark, stepped through from the toolbar button or Cmd/Ctrl+Shift+T, and following the OS live while on System. The choice is held in a store that nothing saves, so as far as the code shows it goes back to System on every launch. `src/lib/stores/theme.ts`.
- **Typography:** Tailwind's prose styles with mdhero's own colors (a cyan accent) and Apple-like fonts; not GitHub's look. `MarkdownRenderer.svelte` (styles).
- **Reader controls** popover: font (sans, serif, mono), text size (14 to 24 px on its slider), line spacing (1.4 to 2.0), code wrap, width (Comfortable or Wide) and the content width (560 to 3840 px). `ReaderControls.svelte`, `settings.ts`.
- **Zoom** is the text size: Cmd/Ctrl+= and -, Cmd/Ctrl+0, Ctrl+mouse wheel and buttons on the home screen, from 10 to 32 px. `+page.svelte`, `src/lib/utils/wheel-zoom.ts`.
- **Zen mode** (Cmd/Ctrl+Shift+F): hides the toolbar, tabs, sidebar and status bar. `+page.svelte`.
- **Full screen** from the View menu. `menu.rs`.

#### Window and UI chrome

- **Native title bar and menu bar:** MDHero (About, Check for Updates, Quit), File (Open, Paste Markdown, Print, Close Tab), Edit (Cut, Copy, Paste, Select All, Find), View (Toggle Theme, Full screen), Window (Minimize, Maximize; not on Linux). `menu.rs`.
- **Window title:** the file name and "MDHero". `files.ts`.
- **Toolbar:** Open, Paste, Open URL, View/Split/Edit, table of contents, reader controls, width, Raw, Present (Marp only), Save, Copy (rich text or Markdown), PDF, Settings and Theme. `Toolbar.svelte`.
- **Status bar:** word count, reading time (230 words a minute) and a token estimate (words times 1.33). `src/lib/components/StatusBar.svelte`.
- **Toasts** for short messages. `src/lib/components/Toast.svelte`.
- **Window size and position remembered** (`tauri-plugin-window-state`). `src-tauri/src/lib.rs`.
- **Home screen:** New document, Browse files, Open URL, Pin folder, recent files, Claude plans, pinned folders' files, zoom buttons and shortcut hints. `EmptyState.svelte`.
- **AI Lookup:** right-clicking the document opens a native menu with Cut, Copy and Paste, Search Google, an "Ask (provider)" submenu of saved prompts per provider, and Custom prompt. Choosing one opens the browser at the provider's URL with the selected text in the prompt. Providers ship for ChatGPT, Claude, Perplexity, Google and Wikipedia, and can be edited, added and removed in Settings. `src/lib/stores/aiLookup.ts`, `commands.rs` (`show_ai_context_menu`), `src/lib/components/AILookupSettings.svelte`, `src/lib/components/CustomPromptModal.svelte`.
- **About dialog.** `src/lib/components/AboutDialog.svelte`.

#### Settings

- **Settings dialog** (Cmd/Ctrl+,): Language, Close on Escape, Restore tabs on launch, Open files in the existing window (Windows and Linux), Tabs on the side, Auto-present Marp, Check for updates automatically; Editor: line numbers, syntax highlighting; AI Lookup providers and prompts. `src/lib/components/SettingsDialog.svelte`.
- Settings, recent files, reading positions, pinned folders and AI providers live in the webview's localStorage; only "open in the existing window" is a JSON file, since Rust reads it before the webview exists. `settings.ts`, `single_window.rs`.

#### Platform and integration

- **File associations:** md, markdown, mdown, mkd. `tauri.conf.json` (`bundle.fileAssociations`).
- **Command line:** one positional file argument. `tauri.conf.json` (`plugins.cli`), `+page.svelte`.
- **Open With and double-click** on macOS through `RunEvent::Opened`, buffered until the page is ready. `lib.rs`.
- **Open files in the existing window** (opt-in, Windows and Linux): a later launch hands its files to the running window as tabs and exits. `single_window.rs`.
- **`mdhero://open?path=...` link** (macOS only), checked strictly: an absolute path to an existing Markdown file. `lib.rs` (`parse_mdhero_url`).
- **macOS Quick Look extension:** Space in Finder shows a `.md` file rendered by the app's own pipeline. `quicklook/README.md`, `quicklook/Preview.swift`, `src/quicklook/preview.ts`.
- **Updates:** once a day a request to GitHub's releases API (can be turned off), a banner or toast, and on "Update now" a signed download, install and relaunch through Tauri's updater. `src/lib/stores/updater.ts`, `src/lib/stores/autoUpdate.ts`, `tauri.conf.json` (`plugins.updater`), `SECURITY.md`.
- **Interface languages:** English, German, Spanish, French and Simplified Chinese, a language picker on first run, and the native menus translated too. `src/lib/i18n/`, `src/lib/components/LanguageDialog.svelte`, `menu.rs` (`set_menu_language`).
- **Network use:** the update check, Open URL, remote images, and AI Lookup opening the browser. No telemetry; versions 0.2.1 to 0.2.10 sent the version and OS family to `mdhero.app`, which was removed. `README.md` Privacy, `SECURITY.md`.
- **Hardening:** DOMPurify on the HTML and Mermaid's SVG, a content security policy, the asset scope above, extension allowlists for read and write, the executable denylist for links. `commands.rs`, `tauri.conf.json`, `MarkdownRenderer.svelte`.

#### Performance claims

- "~8MB" (`README.md` tagline), "~11MB binary" (`README.md` Stack), "Rendering is instant" (`README.md` Why MDHero). The repo holds no measurements or targets. Rendering runs on the page's main thread in one synchronous step (markdown-it, highlight.js including language guessing for unlabeled blocks, KaTeX), and Mermaid is imported up front. The editor's highlighter skips inline scanning on lines over 10,000 characters (`markdown-highlight.ts`, `MAX_INLINE_LINE`); nothing similar was found for the viewer.

#### Planned (README Roadmap)

Quick Switcher (Cmd+P fuzzy finder), folder watch, export to self-contained HTML, presentation mode (beyond the minimal Marp slideshow), compare / diff view.

## 2. Downlight

From `GLOSSARY.md`, `.scratch/downlight-v1/spec.md`, `.scratch/window-design/spec.md`, the issue files' Status lines and `page/src/`.

- **Stack:** Tauri 2; the core crate renders Markdown in Rust with comrak, highlights with syntect and bat's grammars from two-face (ADR 0004), sanitizes with ammonia and decodes with `encoding_rs`; the app crate watches with `notify`; the page is plain TypeScript built with Vite and github-markdown-css, with no UI or CSS framework. Vitest with happy-dom and `cargo test`, rendered fixtures and contract samples shared between the two.
- **Platforms:** Windows x64 only in v1; macOS and Linux are Later.
- **Built** (v1 tickets 01 to 37 closed, 33 wontfix; window design 01 to 09 and 11 to 14 closed): the GitHub look on Mica; GFM, footnotes, alerts, emoji, GitHub heading anchors, front matter as a YAML code block, the raw HTML GitHub allows; highlighting with the **Highlight limit** and copy buttons; decoding with byte order marks, UTF-8 and Windows-1252; **Binary file** refusal, every text file a **Document**; **Local image** through Downlight's own scheme, refreshed when its file changes; **Network share** protection; one process with a window per document; Open, drop and **Notice**; **Reload** at the **Reading position**, the file-gone bar, F5 and Ctrl+R, and a watch that checks itself; link resolution into a **Link action**; **History** with Back and Forward; **Ticks**; **Find** with the **Current match**; Edit in the **Editor** or the **Text file program**, at the reading position's line; zoom of the document only, kept in the **Settings**, and the size of the last window; the page-drawn **Title bar** with snap layouts; **Accent color**, **Theme**, **Backdrop**, **Full width**, **Keep on top** and the settings flyout; the native right-click menu; the per-user installer with PATH and file associations; third-party notices; no network requests of its own.
- **Built, not closed:** ticket 38 (**Wait note** and **Stop**), whose comments say it is built and measured, waits on a hand check (`needs-triage`).
- **Planned:** window design 10, **Counts** (`ready-for-agent`); table sorting 01, **Sort** (`needs-triage`, grilled).
- **Open questions:** window design 15 and 16, the backdrops against Windows' own (`needs-info`).
- **After v1:** ticket 39, the page answering while a large document goes in (`needs-triage`).
- **v1.x** (v1 spec, Out of Scope): full screen; pasting or dropping Markdown text; Copy as Markdown; Outline; printing and PDF; the last 5 documents in the empty window, with no automatic reopening.
- **Later:** tabs; Mermaid and math, loaded only when a document has them; Find options; a table of contents sidebar; the Explorer preview pane; line numbers in the margin; the **Overview ruler** (researched in `.scratch/overview-ruler/research.md`); table sorting; other formats (CSV/TSV, SVG, JSON, XML); a Word, PDF and PowerPoint converter; reader settings; writing ticks to the file; opening non-Markdown local files with their default app; the macOS and Linux ports; a setting to block remote images.
- **Out:** editing in the app; search across files; reading from stdin or `--text`; opening URLs; TOML front matter.

## 3. Comparison

mdhero: yes / partly / planned / no. Downlight: built / planned / v1.x / later / out of scope / no, with the ticket where there is one ("wd" is window design).

### Compared: viewing and rendering

| Feature | mdhero | Downlight | Note |
| --- | --- | --- | --- |
| Where rendering runs | page (markdown-it) | built, Rust core (comrak, ADR 0002) | |
| Tables, strikethrough, autolinks | yes | built | |
| Task lists | partly (shown, can't be ticked) | built, tickable: Ticks (13) | Downlight's ticks belong to the showing, never written |
| Footnotes | no | built | |
| Alerts (`> [!NOTE]`) | no | built | |
| Emoji shortcodes | no | built | |
| Heading anchors as GitHub makes them | partly (own slug, drops non-ASCII letters) | built | |
| Smart quotes and dashes | yes | no | GitHub has none |
| Raw HTML | no (shown as text) | built, GitHub-like allowlist (06) | |
| Front matter | yes, key/value bar | built, as a YAML code block | TOML is out of scope for Downlight |
| Syntax highlighting | yes, 25 languages, guesses unlabeled blocks | built, bat's grammars, no guessing, Highlight limit (07, 31) | PowerShell shows plain in Downlight |
| Copy button on code blocks | yes | built (07) | |
| Wrap long code lines | yes | no | |
| Math | yes (KaTeX) | later | |
| Mermaid diagrams | yes | later | |
| Slides (Marp) | partly | no | |
| Local images | yes, within the folder tree, git checkout or pinned folders | built, own scheme, refreshed on change (32) | |
| Images on network shares | no special handling found | built, never loaded (20) | |
| Remote images | yes | built | Downlight's block setting is later |
| Image lightbox | yes | no | |
| Right-to-left blocks | yes | no | not in Downlight's specs |
| Encodings | UTF-8 only, BOM dropped | built: BOMs, UTF-8, Windows-1252 (05) | |
| Which files open | 7 extensions | built, any text file; binary files refused (34) | |
| Raw source view | yes | no | |
| Line numbers in the margin | partly (raw view and editor) | later | |
| Large documents | no handling found | built: ADR 0003, Highlight limit (31), Wait note and Stop (38, hand check open); 39 after v1 | |
| Other formats (CSV, SVG, JSON) | no | later | |
| Table sorting | no | planned (table sorting 01) | |

### Compared: navigation

| Feature | mdhero | Downlight | Note |
| --- | --- | --- | --- |
| Tabs | yes, top or side, drag, context menu | later | |
| A window per document | partly (a launch opens a new process and window unless opt-in) | built, one process (08) | |
| An open document comes to the front | yes (its tab) | built (08) | |
| Outline / table of contents sidebar | yes | v1.x (Outline), later (sidebar) | |
| Link to another Markdown file | yes, new tab | built, same window; Ctrl+click new window (11) | |
| Link to a heading in another document | no (fragment dropped) | built (11) | |
| Link to another local file | opens it in its default app (PDF, txt, images, mp4) | built, reveals it in Explorer (11); opening it is later, opt-in | |
| Link to a folder | no (the capability doesn't allow it) | built, opens it (11) | |
| Link to a network share | no prompt | built, asks first (20) | |
| Link target on hover | yes | built (11) | |
| Back and Forward (History) | no | built (12) | |
| Vim keys, heading jumps | partly (`n` does nothing) | no | |
| Reading progress bar | yes | no | |
| Scroll to top button | yes | no | |
| Reading position kept across launches | yes | no | Downlight keeps it for Reload and History only |
| Overview ruler | no | later | |
| Quick switcher | planned | no | |

### Compared: find

| Feature | mdhero | Downlight | Note |
| --- | --- | --- | --- |
| Find with count and next/previous | yes | built (14) | |
| F3 and Shift+F3 | no | built | |
| Highlights leave the DOM alone | no (mark.js) | built (CSS Custom Highlight API) | |
| Find in the source or editor | yes | no | Downlight has neither view |
| Find options | no | later | |
| Search across files | no | out of scope | |

### Compared: editing

| Feature | mdhero | Downlight | Note |
| --- | --- | --- | --- |
| In-app editor, split preview, save | yes | out of scope | |
| New document | yes | out of scope | |
| Edit in an external editor | no | built (15), at the reading position's line (35) | |
| Choose the editor | no | built (15, wd 08) | |

### Compared: file handling

| Feature | mdhero | Downlight | Note |
| --- | --- | --- | --- |
| Open dialog | yes (own dialog and the system's) | built (09) | |
| Drag and drop | partly (first file, 5 extensions, others silent) | built, any text file, a Notice otherwise (09, 34) | |
| Paste Markdown text | yes, with LLM unescaping | v1.x | |
| Open URL | yes | out of scope | |
| Recent files | yes, 15 | v1.x (last 5 in the empty window) | |
| Pinned folders | yes | no | |
| Claude Code plans list | yes | no | |
| Session restore | yes | no | the v1 spec says "no automatic reopening" |
| Reload on save, saves by rename | yes | built (10) | |
| Reload keeps the reading position | partly (offset left as is) | built (10) | |
| Reload by hand | no | built, F5 and Ctrl+R | |
| File deleted or moved, and back | no | built (10, 30) | |
| Watch that checks itself | no | built (30) | |
| Close on Escape, quit on the last tab | yes | no | |
| Folder watch | planned | no | |

### Compared: export and sharing

| Feature | mdhero | Downlight | Note |
| --- | --- | --- | --- |
| Print and PDF | yes | v1.x | |
| Copy as Markdown | yes | v1.x | |
| Copy as rich text | yes | no | |
| Copy a selection | yes | built | |
| Export to HTML | planned | no | |
| Compare / diff | planned | no | |

### Compared: look

| Feature | mdhero | Downlight | Note |
| --- | --- | --- | --- |
| GitHub's look | no (own) | built (04) | |
| Theme System / Light / Dark | partly (not remembered) | built, remembered (wd 08) | |
| Accent color | no (fixed) | built, amber or Windows' (wd 03, 09) | |
| Backdrop (Mica, Mica Alt, Acrylic) | no | built (04, wd 08); wd 15, 16 needs-info | |
| Font, line spacing | yes | later (reader settings) | |
| Width | yes, Comfortable or Wide, width slider | built, Full width (wd 06) | |
| Zoom | yes (text size) | built, document only (16) | |
| Zoom level shown | no | built (wd 04) | |
| Zen mode | yes | no | |
| Full screen | yes | v1.x | |
| Keep on top | no | built (wd 07) | |

### Compared: window and UI chrome

| Feature | mdhero | Downlight | Note |
| --- | --- | --- | --- |
| Title bar | native | built, page-drawn with snap layouts (wd 01) | |
| Menu bar | yes | no | Downlight has a native right-click menu |
| Toolbar | yes, about 14 buttons | built, title bar buttons Open, Edit, Full width, Keep on top, Settings (wd 05 to 08) | |
| Window title with the folder | no | built | |
| Document counts | yes (words, reading time, tokens, in a status bar) | planned, Counts in the tooltip, no status bar (wd 10) | Downlight decided against a reading time |
| Selection counts | no | planned (wd 10) | |
| Window size remembered | yes, with position | built, last closed size, offset (16) | |
| Messages | yes (toasts) | built (Notice) | |
| Empty window | yes, a home screen | built, Open and a drop hint (09, wd 14) | |
| Settings UI | dialog | built, flyout (wd 08) | |
| AI Lookup | yes | no | |
| Interface languages | yes, 5 | no | English only; not in the specs |
| About dialog | yes | no | |

### Compared: platform and integration

| Feature | mdhero | Downlight | Note |
| --- | --- | --- | --- |
| Windows | yes, x64 and ARM64 | built, x64 | |
| macOS | yes | later | |
| Linux | yes | later | |
| File associations | 4 extensions | built, 6 extensions (17) | |
| Installer | MSI, NSIS, DMG, deb, rpm, AppImage | built, NSIS per-user, PATH (17) | |
| Command line | yes, one file | built, returns at once (08) | |
| Single instance | opt-in | built, always | |
| Deep link scheme | macOS | no | |
| Preview in the file manager | macOS Quick Look | later (Explorer preview pane) | |
| Auto update | yes | no | the v1 spec rules out an updater |
| Network requests of its own | yes (update check, Open URL) | built, none | |
| Third-party notices shipped | not found | built (36) | |
| Code signing | macOS signed and notarized | no | unsigned in v1 |
| Performance targets and measurements | none | built, timing switch (02, 31) | |

### What mdhero has that Downlight neither has nor plans

- Marp slideshow
- Image lightbox
- Raw source view
- Wrapping long code lines
- Smart quotes and dashes
- Right-to-left blocks (`dir="auto"`, direction wrappers)
- Vim keys and heading jumps
- Reading progress bar and a scroll to top button
- The reading position kept across launches
- Session restore (the v1 spec decided against automatic reopening)
- Pinned folders and the Claude Code plans list
- Copy as rich text
- Zen mode
- AI Lookup on selected text
- Interface languages
- A menu bar and an About dialog
- A reading time and a token estimate (wd 10 decided against both)
- Close on Escape
- Auto update (the v1 spec rules out an updater)
- A deep link scheme
- mdhero's planned export to HTML, compare / diff view, quick switcher and folder watch

What mdhero has that Downlight rules out: editing in the app (editor, split preview, new document, save), Open URL, and (neither has it) search across files.

### What Downlight has that mdhero lacks

- Footnotes, alerts, emoji shortcodes, the raw HTML GitHub allows, and GitHub's look and heading anchors
- Ticks
- History with Back and Forward, and links to headings in other documents
- Edit in an external editor, at the reading position's line, with a chosen editor
- Decoding of UTF-16, Windows-1252 and byte order marks; any text file as a document, binary files refused
- Network share protection for images and links; links that reveal a file or open a folder
- Reload by hand, the file-gone bar and a watch that checks itself; reload anchored to the reading position
- A window per document in one process, with the folder in the window title
- The page-drawn title bar with snap layouts, Keep on top, the accent color, backdrops, and the zoom level shown
- Find with F3 and highlights that leave the DOM alone
- Large documents handled: layout only near the view, the Highlight limit, the Wait note and Stop
- Measured performance targets and the timing switch
- No network requests of its own; third-party notices shipped with the installer
- Planned: Counts (with selection counts), table sorting, the overview ruler

### Candidates worth discussing

A neutral list of where the comparison raises a question, with no recommendation:

- Items already on Downlight's v1.x and Later lists, where mdhero shows one way to build them: Outline and a table of contents sidebar, Mermaid and math, print and PDF, Copy as Markdown, recent files, pasting Markdown text, line numbers in the margin, tabs.
- Items on no Downlight list: an image lightbox, a raw source view, wrapping long code lines, right-to-left blocks, the reading position kept across launches, copy as rich text, jumping between headings by key.

## What I couldn't determine

- Whether mdhero's Open URL reaches GitLab, Bitbucket or other hosts: its `connect-src` lists only GitHub's hosts, which would block them, but I didn't run the app.
- How mdhero's reload behaves in practice: the code replaces the HTML and sets no scroll position, so the offset stays, but whether that lands on the same text wasn't tested.
- That mdhero's theme resets on launch: I found no code that saves it.
- mdhero's binary size, speed and behavior with large documents: the README claims about 8 MB and "instant" rendering, but the repo has no measurements.
- How mdhero's interface does in Windows high contrast or with the keyboard alone: its styles have no `forced-colors` rules, and I didn't run it to check either.
