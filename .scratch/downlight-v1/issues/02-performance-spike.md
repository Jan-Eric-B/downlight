# 02: Performance spike and stack decision

**What to build:** Measure the walking skeleton against the spec's performance targets and decide whether Tauri, comrak and the system webview hold. The app gets a timing switch, off by default, that reports from inside the app the time from the process start to the page's first frame after the document is inserted (**first paint**), and every page task longer than 100 ms. This ticket is the gate: no feature work starts until it passes.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 01 (Walking skeleton)

**Status:** closed

- [x] A timing switch, off by default, reports first paint and page tasks over 100 ms. Later tickets reuse it.
- [x] A generator in the repo writes the test documents: 100 KB and 1 MB of Markdown with a table and a code block about every 30 lines, and 1 MB of prose only.
- [x] Measured on the work machine with Mica on, as the median of 5 runs, with the results written under a "Results" heading in this ticket:
  - [x] cold start to first paint for the 100 KB document (target: under 1.0 s);
  - [x] first paint of a second window in the same process (target: under 300 ms). Until ticket 08 exists, the skeleton may open two files given in one launch;
  - [x] the 1 MB documents: the longest page task (target: the window never stops answering for more than 100 ms) and the time until fully shown (target: under 2 s);
  - [x] memory across all processes, recorded with no ceiling;
  - [x] how much of first paint the read and render of the document take, for the 100 KB and 1 MB documents. Today they start only after the webview has started, loaded the page and run `main.ts`, though none of that work needs the webview. Whether to start them earlier, and how, is decided from the result.
- [x] Mica checked and the findings recorded: it shows in the native title bar; with "Transparency effects" off the background is solid; in high contrast it is solid and readable; whether the window flashes white while it opens. (Title bar and white flash recorded. The transparency effects and high contrast checks moved to ticket 04, which builds the solid background.)
- [x] If cold first paint is at most 1.5 s: ADR 0002 is written, saying why rendering happens in Rust with comrak and not with a JavaScript renderer in the page.
- [ ] If cold first paint is over 1.5 s: work stops here, the results are reported to the user, and the stack is reopened before any other ticket starts. (Does not apply: it is 0.69 to 0.91 s.)

## Results

**2026-11-03, measured** on the work machine: Windows 11 Enterprise build 26200, WebView2 156.0.4314.58, dark mode, Mica on, in everyday use (a browser and VS Code open). The release build (`npm run tauri build`) was measured with `npm run measure`: the median of 5 runs per case, each in a new process, after one discarded run. "Cold" therefore means no Downlight process running; the OS file cache and the WebView2 runtime are warm, as they are for a reader who has opened a document before, but not just after booting. Points in time are ms since Windows created the process. The timing switch takes that from `GetProcessTimes`, so the loader's work before `main` counts.

The numbers move with the machine's load. The tables below are from the last of three measurement sessions, with total CPU load around 20 %. The 100 KB median first paint across the three sessions was 687, 905 and 771 ms. The 905 ms session ran at about 40 % load, and its slowest single run (1024 ms) is the only run over 1.0 s.

| Case | Target | Result |
| --- | --- | --- |
| Cold start, 100 KB, first paint | under 1.0 s | **771 ms** (runs: 705 to 875 ms) |
| Second window in the same process, 100 KB, first paint after the window starts opening | under 300 ms | **214 ms** (203 to 217 ms) |
| 1 MB mixed and 1 MB prose: longest task or frame once the page has the document, while showing it and scrolling through it | never over 100 ms | **none over 100 ms** in any run |
| The page's first frame, before the document arrives, any document | never over 100 ms | **missed**: over 100 ms in about half the runs, 124 to 189 ms; see below |
| 1 MB mixed, fully shown | under 2 s | **767 ms** |
| 1 MB prose, fully shown | under 2 s | **699 ms** |
| Memory, one window, all 7 processes | recorded | 100 KB: 171 MB private, 377 MB working set; 1 MB mixed: 193 / 400 MB; 1 MB prose: 185 / 393 MB |
| Memory, two windows | recorded | 232 MB private, 475 MB working set |

- **Memory:** The working set sums every process's, so pages shared between the WebView2 processes count more than once; private bytes are the better number. The second window adds about 60 MB private.
- **Fully shown** is the first paint after the whole document was inserted: the page inserts it in one step, and from then on every block is in the page. Because of `content-visibility` (below), the browser lays out and paints the blocks out of view when they come into view, not in that frame.
- **The second window** is measured from the moment the running process starts opening it, after the first window's first paint (the `downlight a.md b.md` stopgap until ticket 08). It leaves out a second launch handing its path to the running process, so read 214 ms as a lower bound for what a reader sees.
- **First paint** here is when the page's main thread finishes the frame after the insert (`requestAnimationFrame`, then a task). Compositing and the screen update follow, typically within one more frame.
- The switch watches the page's main thread, as the ticket asks. The app's main thread is not watched.

**Where first paint goes** (medians, ms since the process started):

| Stage | 100 KB | 1 MB mixed | 1 MB prose |
| --- | ---: | ---: | ---: |
| The app starts opening the window | 62 | 63 | 68 |
| The page's navigation starts (the webview has started) | 621 | 589 | 561 |
| `main.ts` runs | 676 | 664 | 625 |
| Read and render start | 691 | 675 | 633 |
| Read and render take | 2.2 | 17.6 | 13.9 |
| The page receives the HTML | 699 | 707 | 660 |
| The HTML is inserted | 701 | 721 | 671 |
| First paint | 771 | 767 | 699 |

- **Read and render are about 0.3 % of first paint for 100 KB and 2 % for 1 MB.** Starting them before the webview is up would save those 2 to 18 ms at most, so they stay where they are: the page asks for the document once `main.ts` runs. About 500 ms of every cold first paint is WebView2 starting between the window opening and the navigation, which Downlight can't shorten. That is the cost ADR 0001's one process avoids for further windows.
- Sending the 1 MB document's HTML to the page takes about 15 ms; inserting it about 13 ms.

**The 1 MB documents needed one change.** Measured as the skeleton was, the frame after inserting the document did style and layout for the whole document. That frame took 186 ms for 1 MB mixed and 141 ms for 1 MB prose; the longest single task was 173 and 132 ms. `content-visibility: auto` on the document's top-level blocks (in `page/src/document.css`, recorded in ADR 0003) lays out only the blocks near the view. With it, the frame after the insert takes 25 to 50 ms for the 1 MB documents, and their first paint got about 70 to 100 ms faster. Things later tickets should know:

- `npm run measure` also scrolls through both 1 MB documents over WebView2's debug port: a jump to the end, a jump to the middle, then 150 page-downs, one per frame. No task or frame went over 100 ms in any of the 10 runs.
- For blocks not yet shown, the document's height is an estimate. Checked once by hand, it was within 1 % of the real height for both documents.
- Reload anchoring (10), heading links (11) and Find's scrolling to a match (14) scroll to blocks that may not have been laid out yet; check that they land where they should. The overview ruler (later) can't place its marks from block offsets until every block has been laid out once.
- The skeleton doesn't render tables yet (ticket 04 turns on GFM), so the "tables" in the mixed documents were paragraphs. Measured once with comrak's table extension on: 1 MB mixed first paint 701 to 728 ms, and no task or frame over 100 ms, also while scrolling.

**The page's first frame misses the 100 ms target.** In about half the runs, a frame of 124 to 189 ms comes before the page has the document (`npm run measure` reports these separately). Each of the four whose start was printed began exactly at the page's navigation. It comes from loading the page itself (github-markdown-css and the script), not from the document: it shows up with 100 KB as often as with 1 MB, and the window shows no document yet. Once the page has the document, nothing went over 100 ms. Hiding the window until first paint (see the white flash below) would keep the reader from seeing it, but wouldn't make it shorter. Ticket 04, which reworks the stylesheet, measures it again and opens a ticket if it stays over 100 ms.

**Mica:**

- **Title bar:** Mica shows in the native title bar. DWM reports the window's backdrop as `DWMSBT_MAINWINDOW` (Mica) with dark mode on, and on screen the title bar and the document share one backdrop.
- **The window flashes white while it opens.** In dark mode the client area is solid white for about 500 ms, from the window appearing (about 70 ms) until the page's first frame (about 650 ms). Setting `WEBVIEW2_DEFAULT_BACKGROUND_COLOR=0` doesn't change it, so the white is not WebView2's default background. Giving the window a transparent `background_color` removes the white, but the window is then see-through (the windows behind show, without Mica) for the same 500 ms. Ticket 04 opens the window hidden and shows it at first paint.
- **Moved to ticket 04: "Transparency effects" off, and high contrast.** Both change settings for the whole desktop, so they are checked by hand, in ticket 04 along with the solid background they test.

## Comments

**2026-11-03, decisions after the spike.** `content-visibility: auto` stays and is recorded in ADR 0003. Ticket 04 takes over the window opening hidden until first paint (no white flash), measuring the page's first frame again, and the transparency effects and high contrast checks.

**2026-11-09, closed.** Closed. The one unticked criterion doesn't apply: cold first paint was 0.69 to 0.91 s, under the 1.5 s at which work would have stopped.
