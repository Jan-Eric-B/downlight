# 07: Syntax highlighting and copy buttons

**What to build:** Code blocks with a language are highlighted in colors that fit light and dark, and each code block has a copy button. Highlighting happens in the core, so the page runs no highlighting code.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 06 (Raw HTML and images, safely)

**Status:** closed

- [x] Fenced code blocks with a known language are highlighted by syntect through comrak's plugin, using the pure-Rust regex feature, with CSS classes rather than inline colors. (Through comrak's plugin interface with an adapter of our own, see Comments.)
- [x] There are colors for light and dark, and a switch in Windows recolors the document without rendering it again. (Checked with emulated media, see Comments.)
- [x] Sanitizing keeps the highlighting classes.
- [x] A code block with an unknown language or none shows as plain code.
- [x] The front matter block is highlighted as YAML.
- [x] Each code block has a copy button that copies exactly the code's text.
- [x] Measured again with the timing switch, with the results recorded in this ticket: the cold first paint target is still met, the 1 MB document still meets its targets, and the growth in binary size is recorded. (Missed at first, 1065 ms; met with render ahead, 760 ms. See Results.)

## Results

**2026-11-10, measured** with `npm run measure` on the release build, median of 5 runs. This ticket's build and the one before it (HEAD, built from `git archive`) ran back to back, so they compare with each other.

| Case | Target | Before | Ticket 07 |
| --- | --- | ---: | ---: |
| Cold start, 100 KB, first paint / shown | under 1.0 s | 633 / 639 ms | **1065 / 1076 ms, missed** |
| Second window, 100 KB, shown after the window starts opening | under 300 ms | 190 ms | 256 ms |
| 1 MB mixed, first paint / shown | under 2 s | 836 / 842 ms | 1740 / 1750 ms |
| 1 MB prose, first paint / shown | under 2 s | 694 / 703 ms | 731 / 739 ms |
| Longest task or frame once the page has the document, also while scrolling | never over 100 ms | none over 100 ms | none in the median; one 1 MB mixed run had one of 117 ms |
| Read and render: 100 KB / 1 MB mixed / 1 MB prose | | 9 / 94 / 39 ms | 410 / 832 / 43 ms |
| Read and render, second window, 100 KB | | 9 ms | 43 ms |
| Private memory: 100 KB / 1 MB mixed / 1 MB prose | recorded | 195 / 226 / 200 MB | 274 / 317 / 201 MB |
| `downlight.exe` | recorded | 5.14 MB | 7.52 MB (+2.37 MB) |

- **Why the cold start misses:** each grammar compiles its patterns on its first use in the process. In a release-mode test: TypeScript 311 ms, Python 42 ms, Bash 36 ms, Rust 30 ms, YAML 6 ms, JSON 1 ms, and under 1 ms on later uses. Loading the grammar set itself takes 2 ms. The test documents use TypeScript, so the first render pays about 400 ms, and the app renders only when the page asks, after the webview has started (about 600 ms in). The second window's 43 ms is the warm cost; highlighting 100 KB itself takes about 35 ms.
- **The 1 MB mixed document** still meets its 2 s, with little room. Warm, highlighting it costs about 400 ms on the background thread.
- **The exe** grew by more than bat's grammar dump (0.94 MB): syntect's parser, fancy-regex and regex-syntax come with it.
- **Memory** grew by about 80 MB private for documents with code, likely the compiled grammars. Prose didn't move.

**2026-11-12, measured again with render ahead** (see Comments), the same way, median of 5 runs.

| Case | Target | Ticket 07 | With render ahead |
| --- | --- | ---: | ---: |
| Cold start, 100 KB, first paint / shown | under 1.0 s | 1065 / 1076 ms | **760 / 770 ms, met** |
| Second window, 100 KB, shown after the window starts opening | under 300 ms | 256 ms | 172 ms |
| 1 MB mixed, first paint / shown | under 2 s | 1740 / 1750 ms | 1064 / 1071 ms |
| 1 MB prose, first paint / shown | under 2 s | 731 / 739 ms | 691 / 698 ms |
| Longest task or frame once the page has the document, also while scrolling | never over 100 ms | none in the median; one run of 117 ms | none over 100 ms in any run |
| Read and render starts: 100 KB / 1 MB mixed / 1 MB prose | | after the webview starts | 75 / 77 / 75 ms |
| Read and render: 100 KB / 1 MB mixed / 1 MB prose | | 410 / 832 / 43 ms | 490 / 881 / 50 ms |
| Private memory: 100 KB / 1 MB mixed / 1 MB prose | recorded | 274 / 317 / 201 MB | 275 / 312 / 197 MB |
| `downlight.exe` | recorded | 7.52 MB | 7.53 MB |

- **The render now runs while the webview starts.** It starts 75 ms in and ends about 565 ms in, before the page's script starts (707 ms), so the page gets the document without waiting for it.
- **First paint is still about 130 ms above the 633 ms from before highlighting.** Reading and rendering takes about 80 ms longer than when it ran alone, so it shares the CPU with the webview's start, which likely makes the webview's start slower by about as much.
- **One 1 MB mixed run had a 101 ms task before the page had the document,** outside the target, which counts only from then on.

## Comments

**2026-11-10, implementation.**

- **Colors are GitHub's own.** github-markdown-css already colors GitHub's `pl-*` classes for light and dark. `core/src/highlight.rs` maps TextMate scopes to those classes in a table, as GitHub's Primer syntax themes do. The table is built into a syntect theme whose "colors" are the classes' indexes. Each colored run is one flat span, and scopes GitHub shows plain get no span. We add no color CSS of our own.
- **Our own adapter** (`GithubHighlighter`) implements comrak's `SyntaxHighlighterAdapter`; comrak's `SyntectAdapter` isn't used. In CSS mode that adapter writes `<pre class="syntax-highlighting">` and drops `data-sourcepos`, so code blocks would lose their line numbers. comrak's `syntect` feature stays off; syntect 5.3.0 (`regex-fancy`, `parsing`) and two-face 0.5.2 (`syntect-fancy`) are direct dependencies.
- **Languages:** bat's grammars from two-face (decided 2026-11-10), looked up by the info string's first word, as an extension or a name, in any case. A block with no language, or one bat doesn't know, is written plain and escaped, with no spans. As on GitHub, nothing is guessed from the first line.
- **PowerShell shows plain.** fancy-regex can't compile its grammar (nor ARM assembly, Salt or Babel's JavaScript). Oniguruma would compile them, but it was turned down on 2026-11-10 to keep the build pure Rust.
- **Rules tuned on screen:** function calls (`variable.function`) are `pl-en`, as on GitHub. A shell command's name and its options show plain.
- **Sanitizing** keeps the classes the table writes, on `span` only, taken from the same table.
- **Copy button** (`page/src/copy.ts`):
  - Each `pre` is wrapped as on GitHub, `div.highlight > pre + div.zeroclipboard-container > button`. The button sits beside the `pre` rather than in it, since a `pre` scrolls sideways.
  - It has no text, only a mask icon and `aria-label="Copy code"`, so Find and a copied selection never pick up its name.
  - It copies `code.textContent` with `navigator.clipboard.writeText` (the page is a secure context, `http://tauri.localhost`), shows a check for 2 s, and shows a failure as a notice.
  - It shows on hover and on keyboard focus.
- **`box-sizing: border-box` on these blocks:** github-markdown-css gives a block with a copy button `min-height: 52px`, meant for github.com's border-box sizing. Without it, short blocks grew by 32 px.
- **For tickets 10 and 14:** a code block's `pre` is no longer a top-level child of the document. Its `div.highlight` is, and that carries `content-visibility` (ADR 0003). The `pre` keeps its `data-sourcepos`.
- **Tests:**
  - `core/src/highlight.rs`: Rust, TypeScript, TOML, unknown and no language, front matter, line number and language class kept, exact text.
  - `core/src/sanitize.rs`: highlighting classes kept on spans only.
  - The new fixture `code.md`.
  - `page/src/copy.test.ts`: one button per block, exact text copied, no text added, the label, a failure shown.
  - Three front matter tests in `core/src/render.rs` checked the unhighlighted HTML exactly. Highlighting the front matter is now required, so they check the same things through the spans.
- **Rendered fixtures:** `github.html`'s front matter is highlighted. `readme.html` is unchanged, since its `sh` command shows plain.
- **Dependencies:**
  - syntect brings bincode 1.3.3, which is unmaintained (RUSTSEC-2025-0141). syntect 5.3.0, the newest release, still uses it.
  - bincode only reads the grammar dump compiled into the exe, so `deny.toml` ignores the advisory with that reason.
  - `cargo audit` gives 6 allowed warnings: the 5 Linux-only ones from before, plus bincode.
  - ADR 0004 records the choice.
- **Not yet done:** the bat grammars' own licenses (two-face's `acknowledgement::listing()`) haven't been listed or checked against GPL-3.0. Ticket 17 ships the notices.

**Checked by hand, 2026-11-10,** on the release build over WebView2's debug port, with `code.md`:

- Keyword, string, comment, entity and tag colors are github-markdown-css's in dark (`#ff7b72`, `#a5d6ff`, ...) and, with light mode emulated, in light (`#cf222e`, `#0a3069`, ...).
- PowerShell and the unknown language show plain.
- 9 blocks have 9 buttons, and the document's text holds no "Copy".
- A click (sent as a user gesture) put the Rust block's exact text on the Windows clipboard, with CRLF line endings, and showed no notice.
- Not checked: a real light/dark switch in Windows, and high contrast.

**The cold first paint target was missed at first.** The options considered, best first:

1. **Render ahead** (recommended). Start reading and rendering a window's document as soon as the window starts opening, in parallel with the webview's start (about 600 ms), and hand the result over when the page asks. The 100 KB document's 410 ms fits inside that wait, so first paint should come back near the 633 ms it was before this ticket, for any language, and every document gains. It changes the app only (`app/src/render.rs`, `app/src/window.rs`), with tests at the app's render seam. Ticket 10's reload keeps rendering on demand.
2. **Warm common grammars** at app start on a background thread (TypeScript, Rust, Python, Bash, JSON, YAML). A small change, but the list is a guess: a document in another heavy grammar still pays.
3. **Oniguruma** instead of fancy-regex. It likely compiles grammars faster (unmeasured) and adds PowerShell, but it adds a C library and was turned down once already.
4. **Show plain first, highlight after.** A fast first paint, but a second render and a swap, which complicate Find, ticks and keeping the reading position on reload.
5. **Relax the target**, or go back to syntect's built-in grammars (no TypeScript).

**2026-11-12, render ahead (option 1), done in this ticket.**

- **The render ahead** is part of the window's entry in `OpenWindows` (`ShownDocument::render_ahead` in `app/src/window.rs`). `OpenWindows::claim` starts it (`render::start`) whenever a window takes a document, a new window or one shown another document, so it runs before the window is built or told. A document that is open already, or a window that has closed, starts none. The page's `render_document` gets it through `OpenWindows::render_of`.
- **Each render ahead serves one request only,** so the next one reads the file again. Ticket 10's Reload renders on demand without further changes.
- **Giving the window another document** replaces its entry and the render with it. A window that closes or fails to open is forgotten, and its render with it.
- **Tests** in `app/src/window.rs`, at `claim`:
  - A window given a new document renders it ahead: the page gets the document as it was read ahead.
  - A window changed to another document renders the new one ahead.
  - The next render after the first reads the file again.
  - Opening an open document again starts no render for its window.
  - A window without a document has nothing to render.
- **Checked:** the first two tests fail when the render ahead is never used.
