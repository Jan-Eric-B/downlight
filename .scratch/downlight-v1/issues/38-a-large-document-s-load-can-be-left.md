# 38: A large document's load can be left

**What to build:** While a large document is on its way, the reader can leave it: the wait note says how long it has been going and what stage it is in, Stop (an "x" on the note, or Esc) returns the window to where it was, and a link, Back or Open goes elsewhere instead of waiting. A window can always be closed, even while its page takes a large document in. Making the page answer during that insert is ticket 39, after v1.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** needs-triage, until the close check below is done by hand.

- [ ] Checked by hand before planning: during a 100 MB mixed load (`npm run documents -- large`), in the insert's freeze, the window closes from the taskbar (right-click, Close) and with Alt+F4. If either fails, making the window close from outside the page is part of this ticket, and a blocker of v1.
- [ ] The wait note says "Showing **a.md** (100 MB) ... 12 s" while the document is on its way: whole seconds, counted from the request, shown from 1 s on. No percentage and no memory figure: neither stage reports progress.
- [ ] When the document's HTML arrives, the note says "Laying out **a.md** ..." and that frame is painted before the page starts taking the document in, so a frozen note says what the window is doing.
- [ ] Stop: an "x" (Fluent's Dismiss icon) at the note's right end, named "Stop (Esc)" as its tooltip and for screen readers, like the title bar's icon buttons. Esc stops too while the note shows, unless something else that Esc closes is open (Find, the settings flyout, a menu), which closes first. Neither is there once the note says "Laying out": the page answers nothing then.
- [ ] Stop puts the window back as it was before the load: the document it showed, at its reading position and its place in History, or the empty window, which then watches no file. A stopped document never joins History. After a link or Open it has no Forward entry; after Back or Forward, History stands where it stood before the step. Its watch keeps running during the load, so a save to it then reloads it.
- [ ] A link, Back or Open during the load goes there instead (the page already drops an overtaken answer), from the document in view: the overtaken document never joins History, as a stopped one doesn't. A on view, a link to B, then a link in A to C while B is on its way: Back from C goes to A. Back during B's load goes to the document before A. Checked by tests in the app and the page.
- [ ] A document joins the app's History only once the page says it placed it (`document_arrived`), not when the window is given it.
- [ ] A save to the file of a document still on its way leaves the render under way and starts a new one; the note's seconds go on counting from the first request.
- [ ] Stopping or leaving tells the left render so, and the core gives up at its next stage boundary (after decoding, after the parse, in the highlighter during formatting, before sanitizing) with no HTML. comrak's parse can't be cut short.
- [ ] Measured with the timing switch, against ticket 31's Results: leaving a 100 MB mixed load ends its render within a few seconds, and the painted "Laying out" frame adds no measurable time to first paint at 20 and 100 MB mixed.
- [ ] Not in this ticket: a reload, which shows no wait note and freezes for about 3 s per save at 20 MB, and the page's freeze during the insert. Both are ticket 39.

## Results

**2026-11-20, measured** with `npm run measure -- large` on the release build of this ticket, on the same machine as ticket 31's Results. Medians of 5 runs at 1 MB and of 3 above, in ms, against ticket 31's "With this ticket" table:

| First paint | 1 MB mixed | 5 MB mixed | 20 MB mixed | 100 MB mixed | 20 MB prose | 100 MB prose |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Ticket 31 | 1306 | 2953 | 7584 | 30901 | 2489 | 13103 |
| This ticket | 1787 | 3069 | 7445 | 31364 | 2218 | 13665 |

- The painted "Laying out" frame adds no measurable time: 20 MB mixed is 139 ms faster and 100 MB mixed 463 ms (1.5 %) slower, within the runs' own spread (100 MB mixed ran 30.4 to 32.4 s). Render, IPC, insert and the frame after it are as in ticket 31 (100 MB mixed: 13.5 s, 2.5 s, 5.5 s, 9.6 s).
- 1 MB mixed rendered slower in this session (1415 ms against 1064), and its first paint with it; the 1 MB documents never show the wait note, so nothing of this ticket runs for them. Ticket 31 saw the same swing between sessions.
- One 20 MB mixed run reported 10.7 GB of working set; the other two 1.3 GB, as in ticket 31. Not reproduced.
- How soon a left 100 MB render ends is read by hand from the `stop` and `render-left` timing lines; it is part of the hand check.

## Comments

**2026-11-19, from the repo owner's hand checks of ticket 31.** Opening a large document (20 MB, 100 MB) freezes the whole window until it shows: the reader can't close it, scroll, follow another link or cancel. The owner asked for a time display in the wait note, perhaps a percentage of what is loaded or the memory it takes, and an "x" that stops the load.

**What freezes, from ticket 31's Results** (100 MB mixed, loaded machine): read 42 ms, render 13.3 s, IPC 2.5 s, insert 5.5 s, frame after the insert 9.6 s, longest page task or frame 11.7 s, memory 3.8 GB.

- The render runs on the app's blocking thread, so the page is free during it (13 s at 100 MB). A link, Back or Open then already overtakes the load: `request()` in `page/src/view.ts` places only the latest answer. The left render runs to its end on its blocking thread and is dropped.
- The insert and the frame after it run on the page's main thread (about 15 s at 100 MB, about 3 s at 20 MB), and ADR 0003's `content-visibility: auto` already skips layout of blocks away from the view. While that thread is busy the page answers nothing, and the title bar is the page's (ADR 0005), so its close button, its drag region and every key are dead too. The app doesn't hold up a close (`CloseRequested` in `app/src/window.rs` only tells the documents table), so the taskbar's Close and Alt+F4 should still work; whether WebView2 lets go of a busy page quickly is the hand check above.

**2026-11-19, grilled with the repo owner.** Decided:

1. **Split.** v1 gets what works while the page is free (the note, Stop, leaving, a cut-short render) and a close that always works. The page answering during the insert is ticket 39, after v1.
2. **The note tells the stage, not a ticking counter alone.** A counter stops while the page is frozen, which reads as a hang: at 100 MB it would count to about 16 and then stand still for 15 s. So the note switches to "Laying out" before the freeze. A compositor-animated bar was rejected: it reads as a progress bar that never fills.
3. **Stop undoes the step that started the load**, by the glossary's Stop: a stopped document was never shown, so it never joins History.
4. **Leaving cuts the left render short** through the highlight limit's own check between lines, since highlighting is most of a mixed document's render.
5. **Esc stops**, after Find's own Esc.
6. **No Stop while laying out**: the page can't answer it then, and a button that ignores clicks is worse than none.
7. **The seconds count from the request**, so they say how long the reader has waited, not how long the note has shown.

**2026-11-20, grilled again with the repo owner, against the code.** Decided:

1. **Decision 4 replaced: a left render stops at its next stage boundary.** Since ticket 31 the highlight limit caps highlighting at 1.5 s per document, so at 100 MB mixed at most 1.5 s of the 13.3 s render is highlighting; the rest is comrak's parse and format and ammonia's sanitizing (`render_within` in `core/src/render.rs`). Ending only the highlighting would leave a left render running about 12 s. So the core looks at whether it was left between its stages and in the highlighter, and gives no HTML. The parse alone can't be cut short; a 100 MB prose render, parse included, takes 4.9 s.
2. **An overtaken document never joins History**, as a stopped one doesn't. Today the app records a document in History when the window is given it (`WindowRecord::given` in `app/src/documents/history.rs`); it now waits until the page says it placed the document, so a Stop during the IPC still works. Going elsewhere during a load starts from the document in view, as in a browser; Stop is the way back to it.
3. **Stop keeps the left-behind document's reloads**: its watch keeps running while it is held, and a change it reports then reloads it, or says it is gone, once Stop puts it back.
4. **A save during the load restarts the render**, the first one left and cut short, and the seconds go on from the first request.
5. **Esc closes what is open first** (Find, the settings flyout, a menu), and only then stops.
6. **The close check is the owner's by hand**, on a release build with the large documents written; it adds scope only if it fails.

**2026-11-20, built.** What changed and where:

- **Core** (`core/src/render.rs`, `core/src/highlight.rs`): `render` takes a `Leave`, a shared flag the app sets once nobody waits for the render. It is looked at after decoding, after the parse, after the tree walks and before sanitizing; formatting writes through a writer that fails once it is set, so comrak stops mid-document; the highlighter looks at it between lines. A left render gives `RenderError::Left` and no HTML. On a test build, a 1 MB document of Rust blocks that renders for 26 s ends at once when left 200 ms in.
- **App, the render** (`app/src/render.rs`): `RenderTask` carries its `Leave` and leaves it when dropped unawaited, as a render ahead the page never took. `DocumentError::Left` keeps the render's stages for the timing switch, and the binary-file error is `DocumentError::NotText`.
- **App, the document on its way** (`app/src/documents.rs`, `app/src/documents/arrival.rs`):
  - `WindowRecord::before_arrival` holds what the window was (a `WindowState`, its document and its history) until the page says it placed its answer for the new document (`document_arrived`, sent for a show, a return and a reload, and since the review for an answer that says why the document can't be shown, and a reload that can't be read). A claim or a step while one is on its way puts that back first and starts from it, so the overtaken document never joins History. A show that fails puts the document on its way back too.
  - `stop_document` puts the held record back and retitles the window. `document_path_shown_as` and `window_showing` count the held document, so a link in it resolves and opening it again brings its window to the front (ADR 0001). A request from the window for the document on its way leaves it on its way and tells the page nothing (`Given::OnItsWayHere`, since the review).
  - The held document's watch keeps running: a change is kept in `ShownDocument::missed` and acted on after Stop. A reload the page hadn't placed when the window was given another document counts as a change.
  - `ShownDocument` keeps the `Leave` of the render it handed over; a later hand-over leaves the one before, and dropping the document (Stop, going elsewhere, closing the window) leaves it.
- **Timing** (`app/src/timing.rs`): `timing <window> stop at=...` when Stop puts the window back (since the review, only then), and `timing <window> render-left at=... read=... render=...` when a left render ends; `at + read + render` against the stop's `at` says how soon it ended.
- **Page** (`page/src/wait-note.ts`, `page/src/view.ts`, `page/src/document.css`):
  - The note counts whole seconds from the request, from 1 s on, in an `aria-hidden` span. A later request for the same file keeps the count; another file starts it again.
  - Once the answer comes, the note says "Laying out **a.md** ...", drops its Stop and is painted in a frame of its own before `prepare` parses the HTML. A request whose note never showed waits for no frame.
  - Stop is an "x" at the note's right end, named "Stop" with "Stop (Esc)" as its tooltip and Escape as its key, one of the page's icon buttons (`iconButton`, since the review), as the find bar's are. It, and Esc when nothing open took it first, drop the request and send `stop_document`; a window that showed no document asks again, once the app answers that it stopped, and shows the empty window. A link to a heading in the document in view and a step within it stop the load first.

Tests:

- Core: `render_left_before_it_starts_gives_no_html`, `render_left_while_it_runs_ends_soon_with_no_html`.
- App (`app/src/documents/tests/arrival.rs`): `stop_puts_the_window_back_at_the_document_in_view_with_its_history`, `stop_in_a_new_window_leaves_it_empty_and_unwatched`, `stop_from_a_page_at_another_showing_changes_nothing`, `stop_once_the_document_arrived_changes_nothing`, `document_joins_history_once_the_page_says_it_arrived`, `link_during_a_load_goes_from_the_document_in_view`, `back_during_a_load_goes_before_the_document_in_view`, `link_in_the_document_in_view_resolves_during_a_load`, `document_in_view_during_a_load_is_still_open_in_its_window`, `save_to_the_document_in_view_during_a_load_reloads_it_after_stop`, `document_in_view_gone_during_a_load_tells_the_page_after_stop`, `reload_overtaken_by_a_load_reloads_after_stop`, `stop_without_a_change_to_the_document_in_view_tells_the_page_nothing`, `second_request_during_a_load_leaves_the_first_render`, `stopping_or_leaving_a_document_leaves_its_render`, `render_the_page_placed_is_not_left`.
- Page (`page/src/wait-note.test.ts`): the count and its screen reader text, a save keeping the count, "Laying out" painted before the document and without Stop, Stop's name, Stop and Esc keeping the document in view, Esc with Find open, Stop in an empty window, a heading link and Back within the document stopping the load, and `document_arrived` after a show and a reload.
- Changed, since a document now joins History only once its page says it arrived: the history tests' helpers and the link and drop tests now have the page place each document (`arrive`, and `Documents::page_asks`, which says it arrived too). `accent.test.ts` lists the Stop's focus ring (since the review, the one `.icon-button` row).

Not in this ticket: Edit and the title bar's reveal during a load act on the window's document, the one on its way, with the line of the one in view, as before. Two quick Back presses while each document is still on its way both go from the document in view, so they take one step, as in a browser.

**2026-11-20, reviewed.** The review found two bugs and four places to restructure; all are fixed.

- **A show or return whose render fails never arrived.** The page sent `document_arrived` only for a render, so after a link to a binary file or a folder named `x.md`, the app held the document before it as in view for good: Back skipped it, its watch ran on unheard, it stayed open in the window, and Stop from the error was refused, so the page asked again and the note came back. The page now tells the app the showing of every answer it placed, an error's too (`answered` in `Placement`), and of a reload that can't be read, which no longer stays unplaced. Stop names the showing of the last answer placed, as the app holds it, and `stop_document` answers whether it stopped (`Stopped`, sampled in `app/tests/fixtures/stop_document.json`): the page asks again only when it stopped and showed no document. The `timing stop` line is written only when Stop stops.
- **Asking for the document in view during a load didn't stop the load.** Ctrl+O or a drop of it gave `AlreadyHere`, and a link from another window to a heading in it found no document to scroll, so the load went on and replaced it. Now a request for the document in view from anywhere, a step from another window included, sends its window `stop-document` with its showing (`Message::StopDocument`), and the page stops as Stop does, also before the note shows; a heading then scrolls in the document put back. A request from the window for the document on its way is its own case, `Claimed::OnItsWayHere` and `Given::OnItsWayHere`, in place of `AlreadyElsewhere` naming the window itself; `WindowRecord::is_on_its_way` replaces the `Option<bool>` comparison.
- **The record's shape.** `WindowRecord` is `now` and `before_arrival`, each a `WindowState` (the document and its history), so a held record can't hold one of its own. `WindowRecord::given` and `stepped` leave the document on its way and hold the one in view themselves (`move_on`), and give what they left to drop after the lock. `WindowRecord::documents` serves `document_path_shown_as` and `window_showing`, which now gives the record too; `put_back` puts the left state back on its way with one swap. "Visit", the review's name for the part, is one the glossary avoids, so it is `WindowState`.
- **One icon button.** `iconButton` (`page/src/icon-button.ts`) makes Find's buttons and the note's Stop alike, with one `.icon-button` class in `document.css`; the copied rules and focus ring are gone, and `accent.test.ts` lists one row.
- **Esc's order.** The view's Esc defers to Find explicitly (`isFindOpen`, besides a key Find already took), so it no longer relies on `main.ts` starting Find first; the settings flyout keeps its Esc to itself. The listener is registered with the view's `signal`, and the tests abort it after each test in place of the `isConnected` guard.
- **Renames.** `RenderTask::leaver`, the page's `stop` (and `stopIfTheReaderCan`, for the note, Esc and going elsewhere in the document in view) in place of `leaveTheLoad`, and `WaitNote.showLayingOut`. `DocumentError::Left` says the reader never sees its notice; `render_document` reports a left render in one error arm; `Leave` and `UnlessLeft` follow `RenderError`'s impls in the core.

Tests added:

- App (`app/src/documents/tests/arrival.rs`): `show_whose_render_fails_arrives_once_its_page_placed_why`, `stop_after_a_failed_show_and_a_further_load_puts_the_failed_show_back`, `failed_reload_its_page_told_of_is_not_reloaded_after_stop`, `stop_tells_whether_it_stopped`, `open_of_the_document_in_view_during_a_load_tells_its_page_to_stop`, `open_of_the_document_on_its_way_leaves_it_on_its_way`, `open_of_the_document_on_its_way_to_a_new_window_leaves_it_on_its_way`, `link_from_another_window_to_a_heading_in_the_document_in_view_during_a_load_stops_it_and_scrolls_there`, `back_in_another_window_to_the_document_in_view_during_a_load_stops_it`, `change_that_fails_during_a_load_puts_the_document_on_its_way_back_with_the_one_in_view_held`.
- Page (`page/src/wait-note.test.ts`): an error placed and a reload that can't be read are told to the app, Stop after an error names that answer's showing and asks nothing again, a Stop the app didn't carry out asks nothing again, the app's `stop-document` stops the load (also before the note shows, and not for another showing), a link from another window to a heading in the document in view stops and scrolls, and Esc with Find started after the view. Contract (`page/src/contract.test.ts`): both `stop_document` answers, and the `stopDocument` message.
- Changed: four tests in `app/src/documents/tests/give.rs` (`document_given_to_its_own_window_is_already_there`, `request_from_the_window_that_shows_the_document_is_given_as_already_here`, `link_to_a_heading_in_the_document_its_own_window_shows_sends_that_window_nothing`, `ctrl_click_on_a_heading_in_the_document_its_own_window_shows_sends_that_window_nothing`) have the page place the document (`page_asks`) rather than only ask for it: their link is clicked in the document, which a page that only asked hasn't placed, so the document was still on its way and is now `OnItsWayHere`. The wait note's "Stop in a window that showed no document" test checks that the empty state shows, not only that it exists.

Not changed: a link to a heading in the document in view, or a step within it, stops the load only while the note shows (`stopIfTheReaderCan`), since before that the page can't tell a load from a reload, which they must not stop.
