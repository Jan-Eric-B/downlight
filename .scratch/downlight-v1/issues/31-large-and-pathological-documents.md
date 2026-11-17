# 31: Large and pathological documents fail gracefully

**What to build:** A document or image too large to show, or a code block too costly to highlight, gives the reader a clear message instead of a blank window, a frozen render or a crash that closes every window.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-human

- [x] Measured first with the timing switch, results recorded here: read, render, IPC and insert for documents of 1, 5, 20 and 100 MB (`npm run documents` can be extended), and the memory used.
- [x] ~~A document over a ceiling chosen from those numbers is not rendered; the view says how large it is and why it isn't shown. The read stops at the ceiling rather than loading the whole file (`app/src/render.rs` reads it whole today).~~ Changed by the owner's decision (Comments): every document is shown, however large. While a document takes over half a second to arrive, the window shows the wait note, "Showing **a.md** (100 MB) ...", instead of an empty or hidden window, until the document replaces it.
- [x] The image scheme serves no file over a ceiling of its own (`app/src/image.rs` reads every image whole). A failed allocation aborts the process, which closes every window (ADR 0001); a downloaded archive can carry a multi-GB zero-filled `x.png` that compresses to almost nothing. The ceiling is 256 MB, well past any real image, so only a pathological file is skipped (it shows as a missing image does, by its alt text), and no more of a file than that is read.
- [x] Highlighting has a budget: a code block over a size (and if needed a time) limit shows as plain code, as GitHub does for large files. Measure the slowest grammars on a large block first. Today a huge block, or input a grammar backtracks on, holds the render thread for as long as fancy-regex takes, and first paint waits for it. (Glossary: Highlight limit.)
- [x] Tested first in the core (the highlight budget) and the app (~~a document and~~ an image over the ceiling, and the wait note's file and size), and in the page (the wait note).

## Results

**2026-11-17, measured** with `npm run measure -- large` (new, see Built) on the release build: Windows 11 Enterprise build 26200, with two other agents building and testing in parallel, so the machine was loaded throughout. Read the numbers against the 1 MB runs of the same session, not against ticket 02's: the 1 MB mixed document took 1286 ms to render here before any change, against 881 ms in ticket 07. Medians of 5 runs at 1 MB and of 3 runs above, in ms; memory is the sum over Downlight and its WebView2 processes, private bytes (working set in brackets), taken 3 s after first paint. The test documents are the generator's: "mixed" has a table and a code block (Rust, TypeScript, Python, JSON) about every 30 lines, "prose" has neither. IPC is from the later of the render's end and the page's request until the page has the HTML. The first session's formula left out the request, which inflated IPC for the small prose documents (in brackets).

**Before this ticket** (the build from `main`):

| | 1 MB mixed | 5 MB mixed | 20 MB mixed | 100 MB mixed | 1 MB prose | 5 MB prose | 20 MB prose | 100 MB prose |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Read | 0.7 | 2.7 | 8.5 | - | 0.7 | 2.0 | 7.8 | 37 |
| Render | 1286 | 3680 | 11388 | - | 47 | 227 | 1020 | 5014 |
| IPC | 39 | 128 | 604 | - | (1335) | (788) | 305 | 1211 |
| Insert | 86 | 355 | 1380 | - | 33 | 79 | 421 | 1952 |
| Frame after the insert | 158 | 398 | 1903 | - | 94 | 157 | 613 | 4283 |
| First paint | 1859 | 4671 | 15347 | none | 1652 | 1335 | 2445 | 12855 |
| Longest page task or frame | 221 | 677 | 2838 | - | 114 | 226 | 927 | 4819 |
| Memory, MB | 359 (594) | 499 (738) | 1164 (1389) | - | 237 (476) | 294 (531) | 575 (801) | 2135 (2349) |

- **100 MB mixed showed nothing within 180 s** in any run: the window stayed hidden for 5 s, then showed empty (the app's fallback) while highlighting went on. Nothing crashed.
- Until a document arrived, its window stayed hidden, up to the 5 s fallback: a 20 MB mixed document's window appeared 5 s in, empty, and its document 10 s after that.
- Highlighting is what makes the mixed documents slow: at 20 MB it is 11.4 of the 15.3 s. For the prose documents the page's own work dominates: inserting and laying out the HTML holds the page for about 50 ms per MB, 4.8 s at 100 MB.

**Highlighting, per grammar** (`core/tests/highlight_cost.rs`, release build, the limits lifted for the measurement): a 256 KB block of lines of about 200 bytes with comments of several kinds, strings with escapes, numbers, brackets, tags and templates, rendered twice in each of the 185 grammars a token finds.

| Grammar | ms per MB of code (second render) |
| --- | ---: |
| syslog | 26886 |
| SystemVerilog | 26144 |
| log | 25281 |
| MATLAB | 23160 |
| TypeScript | 20373 |
| TypeScriptReact | 14795 |
| R | 13800 |
| x86_64 Assembly | 12772 |
| C++ | 11399 |
| SCSS, Sass, C#, Less | 10700 to 11300 |
| YAML | 9392 |
| The median grammar | about 1200 |

- One line of 32 KB costs about the same per MB as many lines (MATLAB 25882 ms per MB, nginx 16283, OCaml 14605), so the grammars measured don't grow worse than linearly with a line's length at that size. A single line is the one thing the time limit can't cut short, since it is looked at between lines.
- A grammar's first use compiles its patterns, as part of the first render: TypeScript's first render of 256 KB took 5038 ms against 5093 for the second, so compiling is small next to highlighting a large block.
- With the limits in place, the same test (now on a 127 KB block and a 3800-byte line, the largest the limits highlight) shows every grammar's block stopping at 1504 to 1520 ms, as plain code for the 12 slowest, and the worst single line taking 157 ms (Command Help).

**With this ticket** (highlight limit, wait note, no document limit), same session:

| | 1 MB mixed | 5 MB mixed | 20 MB mixed | 100 MB mixed | 1 MB prose | 5 MB prose | 20 MB prose | 100 MB prose |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Read | 1.0 | 3.2 | 7.8 | 42 | 0.7 | 3.2 | 8.4 | 40 |
| Render | 1064 | 2111 | 4003 | 13335 | 47 | 232 | 1080 | 4857 |
| IPC | 27 | 121 | 479 | 2531 | 21 | 72 | 243 | 1322 |
| Insert | 50 | 298 | 1255 | 5528 | 16 | 86 | 437 | 2110 |
| Frame after the insert | 87 | 325 | 1707 | 9609 | 49 | 165 | 610 | 4504 |
| Window shown | 1320 | 1601 | 1537 | 1733 | 1065 | 1167 | 2508 | 1545 |
| First paint | 1306 | 2953 | 7584 | 30901 | 1047 | 1156 | 2489 | 13103 |
| Longest page task or frame | 126 | 558 | 2411 | 11698 | 0 | 222 | 955 | 5434 |
| Memory, MB | 341 (579) | 516 (749) | 1049 (1278) | 3771 (4001) | 237 (471) | 291 (526) | 572 (793) | 2146 (2359) |

- **100 MB mixed now shows**, at first paint 31 s after the launch. Its window shows at 1.7 s with the wait note, which stays through the 13 s render and the 2.5 s of IPC. The page then holds for about 12 s while it inserts and lays out the HTML (the longest frame), with the wait note still painted, until the document replaces it. It takes 3.8 GB of memory. 100 MB prose shows at 13 s, its window with the wait note at 1.5 s.
- A document whose answer takes over half a second shows its window with the wait note at 1.5 to 1.7 s, instead of empty after 5 s: here 5, 20 and 100 MB mixed and 100 MB prose.
- 20 MB prose is the case in between. Its render ahead was nearly done when the page asked, so the answer came within half a second and no wait note showed, and the window stayed hidden through the 1 s insert, showing at 2.5 s with the document in it.
- The highlight limit took 20 MB mixed from 11.4 s of render to 4.0 s, and its first paint from 15.3 s to 7.6 s. 1 MB mixed is highlighted whole (its code is well under 512 KB, and its highlighting well under 1.5 s), so the rendered fixtures and the 1 MB test document look as before.

**Normal documents** (`npm run measure`, same session, after the change):

| Measure (median of 5) | 100 KB | 1 MB mixed | 1 MB prose |
| --- | ---: | ---: | ---: |
| First paint | 1040 | 1220 | 959 |
| Shown | 1059 | 1233 | 979 |
| The page's navigation starts (the webview has started) | 837 | 825 | 707 |
| The page receives the HTML | 994 | 1089 | 902 |
| Read and render | 499 | 976 | 47 |
| Longest task or frame once the page has the document | 0 | 0 | 0 |

- The 100 KB first paint is 40 ms over its 1.0 s target in this session, and the cause is the machine's load, not this ticket. WebView2 took until 837 ms to start the page's navigation, against 621 ms in ticket 02, and the page had its document 50 ms after it asked, so no wait note showed and nothing this ticket adds was on the path. It should be measured again on an idle machine (hand checks).
- A second window showed its 100 KB document 272 ms after it started opening (target under 300 ms).
- Scrolling, Find and the reload of the 1 MB documents kept every task and frame at or under 120 ms.
- The reload took 1046 ms from the save, 748 ms of it reading and rendering, so on this loaded machine the wait note showed for about a quarter of a second over the document during a 1 MB mixed reload.

## Comments

**2026-11-13, from the architecture review.** The spec's performance targets stop at 1 MB; nothing says what happens beyond. A file opened from the command line is rendered whatever it is (`downlight some.bin`), so the ceiling matters for more than Markdown; ticket 34 asks what the command line does with files that aren't Markdown.

**2026-11-17, decided by the repo owner.** No document size limit: every document shows, however large. A large one shows that it is on its way (the wait note). Images are guarded only against pathological sizes. The highlight limit is kept. (A first build had a 25 MB document limit, with "Can't show **a.md**: it is 120 MB, over the size limit ..." in the document's place; it was taken out before it was committed.)

**2026-11-17, built.** What changed and where:

- **Highlight limit** (`core/src/highlight.rs`, `HIGHLIGHT_LIMIT`, glossary: Highlight limit):
  - A code block over 128 KB, or with a line over 4 KB, shows as plain code.
  - One document highlights at most 512 KB of code and spends at most 1.5 s highlighting. A debug build, which the tests and `npm run tauri dev` run, gets 30 s instead, since it highlights many times slower.
  - Once either is spent, that block and every block after it show plain, so plain code starts at one place.
  - Why these values, from the Results: the median grammar takes about 1.2 s per MB and the slowest over 20 s. So 1.5 s of highlighting leaves a 1 MB document its 2 s to be fully shown even with a slow grammar. 512 KB of code is about three times what the 1 MB test document has, so documents up to a few MB are highlighted whole, and a larger one turns plain at the same place on every machine.
  - The time is looked at between lines, so the line limit bounds how far one line runs past it: the worst 3800-byte line took 157 ms.
  - `render` keeps its interface. `render_within` takes the limit, so the core's tests use a small one.
- **Wait note** (glossary: Wait note):
  - The page (`page/src/view.ts`) starts a 500 ms timer with each request for its document.
  - If the answer hasn't come by then, it asks the app's new `wait_note` command (`app/src/documents/handover.rs`). The command gives the window's document's file and its size as Explorer shows it ("2.3 MB"), or no size when the file can't be looked at.
  - The page shows "Showing **a.md** (2.3 MB) ..." as a bar at the top of the view (`.wait-note` in `page/src/document.css`), over the document in view if there is one, until the latest request ends.
  - The window is built hidden and shown once its first frame is drawn (ticket 04). It now also shows with the wait note, so a large document's window appears at about 1.5 s instead of after the 5 s fallback, empty. First paint is still reported only at the document's frame.
- **Images** (`app/src/image.rs`, `LARGEST_IMAGE`):
  - The image scheme reads a local image with `disk::read_no_larger_than` (`app/src/disk.rs`). It looks at the file's size first, and reads at most one byte past 256 MB whatever the size said, since a file can grow while it is read.
  - A larger file is Not Found, and shows as a missing image does.
  - Why 256 MB: the largest real images in documents, such as 100-megapixel scans and panoramas, stay under it, and reading 256 MB can't fail to allocate on any machine Downlight runs on. A file of many GB could abort the process.
- **Measurement tooling:**
  - `npm run documents` also writes 5, 20 and 100 MB of each kind (`scripts/generate-documents.js`).
  - `npm run measure -- large` measures the 1 MB documents and those, with three runs each beyond 1 MB and up to 180 s each. A case that fails is reported rather than stopping the rest (`scripts/measure.js`).
  - `core/tests/highlight_cost.rs` is an ignored test that prints each grammar's cost (`cargo test --release -p downlight-core --test highlight_cost -- --ignored --nocapture`).
- **A fix to `npm run measure`'s cleanup:**
  - What went wrong: the first measurement run left a Downlight process alive. `taskkill /T /F` failed when a WebView2 process in the tree exited on its own, and the script went on after 1 s. The next launch was then handed to that process, or its webview failed with "The requested resource is in use" (ERROR_BUSY) in an error box that waited for a click on the owner's desktop.
  - The fix: the script now ignores taskkill's failure. Before every launch and after every run, it waits until no `downlight.exe` and no WebView2 process on Downlight's user data folder is left. If one stays for 60 s it stops the measurement, and it ends no process it didn't start.
  - The 20 MB document itself left nothing hanging: in the clean runs it showed, and the process ended as any other.
- **Found on the way:** a window whose webview fails to start leaves an empty window behind after the error box is dismissed. The process keeps running and the title bar does nothing. Written up as ticket 37; not fixed here.

Tests, each written before its code:

- Core (`core/src/highlight.rs`):
  - `code_block_over_the_highlight_limit_shows_as_plain_code`
  - `code_block_over_the_apps_highlight_limit_shows_as_plain_code_with_its_text`
  - `code_block_with_a_line_over_the_highlight_limit_shows_as_plain_code`
  - `code_blocks_after_the_documents_highlight_limit_is_spent_show_as_plain_code`
  - `code_blocks_show_as_plain_code_once_the_documents_highlight_time_is_spent`
- App:
  - `app/src/image.rs`: `image_as_large_as_the_largest_served_is_served`, `image_larger_than_the_largest_served_is_not_served_and_not_read_whole`
  - `app/src/disk.rs`: `read_of_a_file_that_never_ends_stops_past_the_largest_size`
  - `app/src/documents/handover.rs`: `wait_note_names_the_documents_file_and_its_size`, `wait_note_for_a_file_whose_size_cant_be_told_names_no_size`, `sizes_are_told_in_explorers_units`
  - The new contract samples, `app/tests/fixtures/wait_note.json`.
- Page:
  - `page/src/view.test.ts`: a document slower than half a second shows the wait note naming its file and size until it arrives; its window shows with the note; a document that comes sooner shows none; one given while another is in view shows the note over it.
  - `page/src/contract.test.ts`: each `wait_note` sample through the page.

Decisions taken with a default, for the owner to overrule:

- **The wait note shows for any request slower than 500 ms, a reload too.** The page can't tell a reload from a show until the answer comes. On this loaded machine that put the note over the document for about a quarter of a second during a 1 MB mixed reload. On an idle machine a 1 MB reload stays under 500 ms.
- **The note gives the size as Explorer does:** 1024-based KB, MB and GB, with one decimal below 10.
- **The page's insert of a very large document still holds the page,** about 12 s at 100 MB mixed and 5 s at 100 MB prose. The owner accepted this: the wait note stays painted until the document replaces it.
- **A document just too fast for the note keeps its window hidden while the page inserts it,** as 20 MB prose did (shown at 2.5 s). The rule is time from the request, as the owner preferred, and the insert comes after the answer.
- **Several large local images are read at once,** each up to 256 MB, so a document naming many huge images can still take a lot of memory. No file is skipped for that.

Hand checks for the owner:

- Open a 20 MB and a 100 MB document from the file manager (`npm run documents` writes `target/documents/mixed-20mb.md` and `mixed-100mb.md`). The window should appear within about 2 s with "Showing **mixed-100mb.md** (100 MB) ...", stay still while the page takes the document in, and then show the document in place of the note.
- Follow a link to a large document from a small one: the note shows over the small document until the large one replaces it.
- A document with a code block over 128 KB in a slow grammar (TypeScript) shows that block as plain code, and the blocks before it highlighted.
- A local image larger than 256 MB shows as missing (its alt text) and the window stays open; a normal large image (20 to 100 MB) loads.
- Measure the 100 KB first paint again on an idle machine (`npm run measure`): this session's 1040 ms came with a loaded machine.
