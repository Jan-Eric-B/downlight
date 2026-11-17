# 24: The documents module sends a link's scroll where it decides it

**What to build:** A refactor of the app, with no change the reader sees. When a link names a heading in a document that is open already, `Documents::give` decides whether that window's page scrolls now or gets the heading with its first render, and it now also sends the `scroll-to-heading` itself, through its `Pages`, instead of handing the heading back through `Given::AlreadyOpen` and `Shown::AlreadyOpen` for `links::show_document` to send two modules away. The link's own window still scrolls through the command's own result, and never gets the message.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 23 (The app decides whether a page shows or reloads its document)

**Status:** closed

- [x] `Documents::give` sends `Message::ScrollToHeading` to the window that shows the document already, when the document came with a heading, that window's page has asked for its document, and it isn't the window the request came from. A failure to send it is ignored, with the reason next to the send.
- [x] `scroll_to` leaves `Given::AlreadyOpen` and `Shown::AlreadyOpen`; `links::for_another_window` and its tests go.
- [x] A link in a window to the document that window shows scrolls through `Followed::ScrollTo` (the heading or the top), with a plain click and with Ctrl+click, and that window gets no `scroll-to-heading`.
- [x] A link to a document open in another window without a heading leaves that window at its reading position, and a page that hasn't asked yet still gets the heading with its first render.
- [x] Open (Ctrl+O), dropping and launches keep their call sites as they are.
- [x] Tested first, in `app/src/documents/tests/give.rs` through `give` and the recorder.

## Decisions

**2026-11-12, from the architecture review after ticket 23.**

- **Why:** `Documents::claim` decides between holding the heading and scrolling now, but the send happened in `links::show_document`, after `window::show` had copied the decision into `Shown`. The helpers `for_another_window` and `left_for_the_page` were pulled out to be testable, while the send itself had no test. Deciding and sending in one place puts the whole rule behind `give`'s interface, where the recorder sees it.
- **The window the request came from:** a variant on `Target`, `Target::NewWindowFrom(label)` for a Ctrl+click, rather than a field on `DocumentAt` or a parameter on `give`. A request for `Target::Window(label)` (a link, Ctrl+O, a drop) always comes from that window, so only a new window needs to say where it came from, and only a Ctrl+click has such a window; launches keep `Target::NewWindow`. A separate field or parameter could disagree with the target or be left out, and every call site would have to fill it; the variant can't be either, and the match in `claim` must handle it. The window a request comes from never gets the message: its page scrolls on the link's own result, and Ctrl+O and drops come with no heading.
- **Order:** the message is now sent before `window::show` brings the window to the front, where before it was sent after. The page scrolls when the event runs either way, and a minimized window keeps its layout, so the order makes no difference to the reader.

## Comments

**2026-11-13, done.** `Documents::claim` returns the heading to scroll to now beside `Given`, leaving it out for the window the request came from (`Target::from`), and `Documents::give` sends `Message::ScrollToHeading` after the lock, ignoring a failure for the reason `links::show_document` gave before. `Given::AlreadyOpen` and `Shown::AlreadyOpen` hold only `window`. `Target::NewWindowFrom(label)` is the new variant for a Ctrl+click, which `links::destination` returns; launches keep `Target::NewWindow`, and Ctrl+O and drops keep `Target::Window`. `links::show_document` only maps `Shown` to `Followed` through `left_for_the_page`; `for_another_window` is gone.

Tests: `link_to_a_document_open_in_another_window_scrolls_it_only_with_a_heading` (now checks the recorder), `link_to_a_heading_in_the_document_its_own_window_shows_sends_that_window_nothing`, `ctrl_click_on_a_heading_in_the_document_its_own_window_shows_sends_that_window_nothing`, `ctrl_click_on_a_heading_in_a_document_open_in_another_window_scrolls_that_window`, `ctrl_click_on_a_document_open_nowhere_opens_a_new_window`, `scroll_that_cannot_be_sent_still_gives_the_window_that_shows_the_document` and `heading_for_an_open_window_whose_page_has_not_asked_is_held_for_its_first_render` (now also checks that nothing is sent) in `app/src/documents/tests.rs`; `ctrl_click_shows_the_document_in_a_new_window` (updated) in `app/src/links.rs`. Removed with `for_another_window`: `link_to_a_heading_in_a_document_open_in_another_window_scrolls_that_window`, `link_to_a_heading_in_this_window_sends_no_window_anything`, `link_without_a_heading_leaves_the_other_window_at_its_reading_position` and `link_that_opened_its_document_sends_no_window_anything`.

**2026-11-13, after the review of tickets 22-24.** The names above and in "What to build" are those of the time. `Given::AlreadyOpen` and `Shown::AlreadyOpen` are now split into `AlreadyHere` (the window the request came from) and `AlreadyElsewhere`, and `Target::from` is gone: `claim` matches on `Target` itself. The `app/src/documents` tests named above are now in `app/src/documents/tests/give.rs`. See ticket 22's comments.

**2026-11-16, closed by the repo owner.**
