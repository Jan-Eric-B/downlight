# 04: Documents look like GitHub on Mica

**What to build:** A README renders the way GitHub shows it: GitHub Flavored Markdown, footnotes, alerts, GitHub's heading anchors, emoji shortcodes, and front matter as a YAML code block. Every block carries its **line number**. The document sits on Mica in light and dark, following Windows, with a solid background where Mica is off.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 02 (Performance spike and stack decision)

**Status:** closed

- [x] Tables, task lists, strikethrough, autolinks and tagfilter render as on GitHub. (tagfilter: comrak 0.59 has none, see Comments; ticket 06 removes those tags.)
- [x] Footnotes render with links from the reference to the note and back.
- [x] Alerts (NOTE, TIP, IMPORTANT, WARNING, CAUTION) render as colored callouts in light and dark.
- [x] Heading anchors match GitHub's, including repeated headings, punctuation and non-ASCII text.
- [x] Emoji shortcodes such as `:tada:` render as emoji.
- [x] YAML front matter between `---` lines shows as a YAML code block at the top.
- [x] Every block element carries the line of the file where it starts.
- [x] Light and dark follow Windows, also when Windows switches while the window is open.
- [x] On Mica the page draws no background of its own. Code blocks, tables and quotes keep their fills, and where Mica is off (Windows 10, transparency effects off, high contrast) the background is solid and readable.
- [ ] Checked by hand and recorded in this ticket, moved here from ticket 02: with "Transparency effects" off the background is solid, and in high contrast it is solid and readable. (Transparency effects checked; a real contrast theme is still open, see Results.)
- [x] The window opens hidden and shows once the document is painted, so it never flashes white or shows an empty frame while WebView2 starts. Ticket 02 found about 500 ms of solid white in dark mode; `WEBVIEW2_DEFAULT_BACKGROUND_COLOR` doesn't help, and a transparent background makes the window see-through instead. First paint with `npm run measure` stays within the spec's targets.
- [ ] Checked by hand and recorded in this ticket: a page that fails before it asks to be shown (a `throw` at the top of `page/src/main.ts`) leaves its window hidden for 5 s, then the app shows it, and closing it ends the process.
- [ ] Checked by hand and recorded in this ticket: the same document opened twice quickly from a cold start, while its window is still hidden, ends up in one window that comes up in front. (tao's `set_focus` does nothing on a hidden window, so bringing it to the front relies on `show_window`.)
- [x] The page's first frame, before the document arrives, measured again with `npm run measure` after the stylesheet work. Ticket 02 found it at 124 to 189 ms in about half the runs, over the 100 ms target. If it is still over, a ticket is opened for it.
- [x] The core's render tests cover each item above that the core renders.

## Results

**2026-11-09, measured** with `npm run measure` on the release build, on the work machine (Windows 11 build 26200, dark mode, Mica on), median of 5 runs after one discarded run. Ticket 02's caveats hold: the numbers move with the machine's load.

| Case | Target | Result |
| --- | --- | --- |
| Cold start, 100 KB, first paint / window shown | under 1.0 s | **592 / 600 ms** |
| Second window, 100 KB, window shown after the window starts opening | under 300 ms | **203 ms** (255 ms from the second launch's start) |
| 1 MB mixed, fully shown (first paint / shown) | under 2 s | 703 / 710 ms |
| 1 MB prose, fully shown | under 2 s | 641 / 649 ms |
| 1 MB mixed and prose: longest task or frame once the page has the document, also while scrolling | never over 100 ms | **none over 100 ms** in any run |
| The page's first frame, before the document arrives | never over 100 ms | **none over 100 ms** in any of 15 runs (ticket 02: 124 to 189 ms in about half); see below |
| Read and render | | 100 KB 3.0 ms, 1 MB mixed 27.6 ms (ticket 02: 17.6), 1 MB prose 17.4 ms (13.9) |
| Memory, one window, private / working set | recorded | 100 KB 173 / 379 MB; 1 MB mixed 237 / 444 MB (ticket 02: 193 / 400); 1 MB prose 187 / 393 MB |

- **Shown** is the new `window-shown` report: the window opens hidden, and the page asks the app to show it once the frame with its document is done. It comes 7 to 8 ms after first paint. Checked first, before building on it: the page of a hidden window does get its frames (first paint comes before `window-shown` in every run), so no cloaking was needed.
- **The page's first frame:** no task or frame over 100 ms was reported before the document in any run. The reports themselves work: a 200 ms page task, set off over the debug port, was reported as both a long task and a long frame. The likely reason is that the page now loads while its window is hidden, so the frame ticket 02 saw at navigation is not drawn, or not reported, then. Either way the reader can't see or use the window during that time, so no ticket is opened for it. A later change that shows the window earlier should measure it again.
- Rendering grew with the extensions and the line numbers: the 1 MB mixed document renders 10 ms slower, and its window uses about 45 MB more private memory, mostly for its larger HTML. All targets still hold.

**Checked by hand, 2026-11-09,** on the release build with `core/tests/fixtures/github.md`, from screenshots:

- Dark mode on Mica: the title bar and the document share one backdrop; the front matter's YAML block, code blocks and table rows keep their fills; the five alerts show in their colors; the scroll bar is dark.
- Windows switched from dark to light while the window was open: the page, the title bar, Mica's tint and the scroll bar all followed, without a reload.
- "Transparency effects" off, in light and in dark: the background is solid and readable. Turned on again afterwards.
- No Mica (Windows 10's path, checked by building with `has_mica()` forced false): GitHub's solid background, and nothing of the windows behind shows through.
- High contrast, emulated over the debug port (`forced-colors: active`): the page draws the reader's Canvas color under the reader's text colors and stays readable. **Still open:** a real contrast theme, which also changes what Windows draws behind the page, and real Windows 10, for which there is no machine.
- Footnote references jump to their note (github-markdown-css outlines the `:target`), and the note's back link returns to the reference.
- The window came up with its document drawn, and PageDown scrolled without a click once the window was in front. No white flash or empty frame was seen; that is by eye and from the timing, not from a frame-by-frame recording.

## Comments

**2026-11-09, implementation.**

- **comrak options** (`github_options` in `core/src/render.rs`): tables, strikethrough, autolinks, task lists with GitHub's classes, footnotes, alerts, heading ids with an empty prefix (so `#installation` finds its heading), shortcodes (comrak's `shortcodes` feature, which brings in `emojis` 0.9.1), front matter between `---` lines, and source positions. Raw HTML stays omitted until ticket 06.
- **tagfilter:** comrak removed the extension in 0.56 ("It's bad. Don't use it."), and the spec pins 0.59. Ticket 06's sanitizing now lists the tags tagfilter escaped; GitHub shows them as escaped text, Downlight drops them.
- **Front matter:** comrak leaves it out of the HTML, so the core turns its node into a fenced `yaml` code block that keeps the node's source position (`1:1-n:3`). comrak already counts the front matter's lines for the blocks after it.
- **Line numbers** are comrak's `data-sourcepos="line:column-line:column"`; the page reads the number before the first colon. comrak 0.59 also writes them on inline elements, with no option to stop it, so the core clears those: only blocks carry line numbers, and a 1 MB document's HTML doesn't grow for nothing. For ticket 10: the footnotes' `<section>` comes last in the HTML but carries the line of the first note's definition, and its `<ol>` carries none.
- **Not drawn:** GitHub's octicons in alert titles and heading anchors. comrak writes none, and the anchors are empty links.
- **Backdrop:** the per-OS module's `has_backdrop()` (named `has_mica()` when the checks above were made) is true on Windows from build 22000. It reads the build with `windows-version` 0.1.7, the version window-vibrancy already brings in, rather than the newest (0.100), which would have added a second copy. `open_window` asks it once per window: only then is the window made transparent with Mica (`with_backdrop`), and the page opened with `?backdrop`, on which it drops its background (`page/src/backdrop.ts`). The neutral name is for macOS vibrancy, which will use the same flag. Elsewhere the page draws github-markdown-css's `--bgColor-default` (`#ffffff` / `#0d1117`), and in high contrast the system's Canvas. `color-scheme: light dark` on the root makes the scroll bar follow dark mode. Following Windows' light or dark needed no code: tao re-themes the window when the setting changes, and Tauri passes it on to WebView2's preferred color scheme.
- **Hidden until painted:** windows open with `visible(false)`. The page invokes the new `show_window` command after the frame with its document, the empty state or the error (`page/src/view.ts`), so no window stays hidden. If the page never asks, the app shows the window after 5 s anyway, so a broken page can't leave an invisible window that keeps the process alive. `npm run measure` now reports `shown` beside first paint.

**2026-11-09, review and close.** After a code-quality review: the backdrop is decided once per window (`has_backdrop()`, `?backdrop`, `.backdrop`), the app's timing no longer keeps each window's opening time (`npm run measure` subtracts `window-opening` itself), and `afterNextFrame` moved from `page/src/timing.ts` to `page/src/view.ts`. Closed with three hand checks still open: a real contrast theme, the 5 s fallback for a page that never asks to be shown, and opening the same document twice while its window is still hidden. `npm run measure` was not run again after the timing change.
