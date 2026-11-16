# 28: The documents module carries out the whole show

**What to build:** A refactor of the app, with no change the reader sees. `Documents::give` decides what a show does, and now also carries it out: opening the new window, titling a window given another document, bringing a window to the front, and undoing the show when part of it fails. The window side goes through a small port beside `Pages`, with Tauri in the app and the recorder in the tests, so the whole show is tested without a webview.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] A windows port (roughly: open a window, set its title, bring it to the front) sits beside `Pages`. Tauri's `AppHandle` is the app's adapter and the recorder the tests' adapter, as ticket 22 decided for `Pages`: a trait with two adapters, not code generic over `R: Runtime`, and no mock runtime.
- [ ] `window::show`, `Shown`, `open_or_forget` and `links::left_for_the_page` fold into the documents module, or go; `links::follow` maps what `give` did to `Followed` directly.
- [ ] What a new window starts with is decided in one place: the page URL and its flags (`timing`, `backdrop`), the navigation guard, the backdrop, the key setup and the 5 s fallback. `Timing::page_url` moves out of the timing module, which only times. Ticket 16 adds the zoom, size and offset there.
- [ ] Ticket 22's open item 6: a give whose page can't be told restores the window's previous entry, so the table and the window agree. Tested with the recorder.
- [ ] Ticket 22's open item 7: tests that read the table's internals (`wait_for_render_ahead`, `opening_an_open_document_again_starts_no_render_for_its_window`) check what the recorder sees instead; `watch_that_cannot_start_is_noticed_once_the_page_asks` proves the hold.
- [ ] `open.rs`'s drop notice and `menu.rs`'s menu command go through the documents module's `Pages`, not straight through `AppHandle`, so the recorder sees every message a page gets.
- [ ] Errors the reader sees no longer carry `tauri::Error`: a documents error type says what went wrong in the reader's terms. Today `Given::WindowGone` reaches the reader as "window not found", through `OpenError::Show`, `FollowError::Show` and `LaunchFailure`.

## Comments

**2026-11-13, from the architecture review.** Following a link to a document runs through `Target`, `Claimed`, `Given`, `Shown`, `Followed` and `Place`; `Shown` only re-maps `Given` for one caller. The pure helpers around the orchestration are tested (`destination`, `left_for_the_page`, `dropped_document`, `item_id` and `chosen`), but their callers aren't, since they take a concrete `AppHandle`. Ticket 12's Back must retitle the window and ticket 16's new window takes a size and an offset; both become testable here.

**2026-11-16, from window design ticket 01.** Part of the third checkbox is done: `page_url` and `WindowLook` moved from `timing.rs` to `window.rs`, beside `open_window`, and `Timing` only says whether timing is on (`Timing::is_on`). The flags now include `titleBar` (the page draws the title bar in a frameless window), and they have a contract sample (`app/tests/fixtures/page_url.json`, every flag on and none), which the page's `pageFlags` in `page/src/app.ts` is checked against. What is left is gathering the rest of what a new window starts with in one place.

**2026-11-16, from ticket 29.** The key setup of a new window is now `platform::leave_keys_to_the_page(&window)` in `open_window` (`app/src/window.rs`), right after `build()`: it turns WebView2's browser keys off and adds the one `AcceleratorKeyPressed` handler. It returns nothing and never fails the window, so a windows port that records what a window starts with can record it as one step with no outcome.

**2026-11-16, from ticket 26.** `Documents::reload` and the `reload_document` command are gone (F5 asks the page's view directly), and `Documents::handover` sends nothing now: a held notice goes in the answer. Every message a page gets from the documents module is `AskForDocument`, `Gone(Showing)`, `ScrollToHeading` or `Notice`. `links::follow` looks the document up with `Documents::document_path_shown_as(window, showing)`, since `follow_link` now takes the showing whose document the link was in; folding `follow` into the documents module keeps that check.
