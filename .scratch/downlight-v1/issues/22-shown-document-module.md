# 22: One module owns a window's shown document

**What to build:** A refactor of the app, with no change the reader sees. One module owns everything about the document a window shows: giving it to the window, watching its file, rendering ahead, holding notices and the heading until the page asks, reloading it only while the window still shows it, and forgetting it. App-to-page messages go through one typed adapter, so this lifecycle can be tested without a webview.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 21 (A save never reloads a document the window has just left)

**Status:** ready-for-agent

- [x] One module's small interface (roughly: give, reload, hand over to the page, forget) replaces the eight `OpenWindows` methods that callers chain today with a (label, key) pair, and `app/src/reload.rs` folds into it.
- [x] Every message the app sends a page (`document-shown`, `document-changed` with `reload` or `gone`, `scroll-to-heading`, `notice`, `menu-command`) goes through one typed set of messages, with Tauri's `emit_to` as the app's adapter and a recorder as the tests' adapter.
- [x] Tests drive the lifecycle through that interface and check the messages sent: a document given and then changed, a reload overtaken by another document, a notice held until the page asks, a window that closes while its render is running.
- [x] The event names and payloads the page receives stay as they are, so the page and its tests don't change.

## Decisions

**2026-11-12, triage.**

- **When:** before ticket 12. Back and Forward gives a window a document in one more way, so it should be built on this module rather than add another caller to `OpenWindows`. Ticket 12 is blocked by this ticket.
- **Scope:** the shown-document module and the typed app-to-page messages, together; the messages are what make the module testable. Splitting the rest of `window.rs` (window building, launch failures, titles) is not part of this ticket.
- **Test seam:** a small messages trait that the module sends through, with `emit_to` in the app and a recorder in the tests. The code isn't made generic over Tauri's `R: Runtime`, and the tests don't use Tauri's mock runtime. Two adapters justify the seam.

## Comments

**2026-11-12, from ticket 14's architecture review.** Why this is worth doing, and other leads from the same review.

The problem: per-window state lives in `ShownDocument` in `app/src/window.rs` (render ahead, watch, `page_asked`, held notice, heading), and `window.rs`, `reload.rs` and `render.rs` each drive part of it. To follow "a window is given a document" you go through `show`, `claim`, `ShownDocument::given`, `render::start`, `reload::watch_document`, `Watch::start`, `keep_watch` or `hold_notice`, and `notice::show_notice`. `window.rs` and `reload.rs` import each other. `reload.rs` is 68 lines of glue: deleting it would move its code, not remove any. Ticket 21's race comes from this shape, two calls where one was needed. Because the commands and `show` take a concrete `AppHandle`, the orchestration (`show`, all of `reload.rs`, the notice ordering in `render_document`, `links::show_document`, `open::show_when_dropped`) has no tests; `app/src/window/tests.rs` tests `claim` directly and polls the table's internals to see a render finish.

Related: `render_document` can't tell a show from a reload. The heading goes out with every render and the page drops it on a reload, so the app and the page each hold half of that protocol. `page_asked` with `held_notice` and `page_asked` with `heading` are two copies of the same "hold it until the page asks" step. The typed messages would make both visible in one place.

Other leads, smaller and not part of this ticket unless it touches them:

- `window.rs` also builds the Tauri window (navigation guard, backdrop, the 5 s fallback for showing it, the key handler), holds the launch failures and their report, and formats titles; its tests mix titles and failure reports with the table's logic.
- `app/src/links.rs` tests its pure helpers (`destination`, `left_for_the_page`), but not `show_document`'s `window != label` check or its `scroll-to-heading` message.
- `OpenError`, `FollowError` and `LaunchFailure` carry `tauri::Error` from `show`, and `Claim::WindowGone` becomes `tauri::Error::WindowNotFound`, so the reader can see "window not found" where the window had just closed.
- Speculative, inside the core: sanitizing takes two calls that must run in order, `drop_filtered_tags` on the tree and then `sanitize` (`core/src/sanitize.rs`, called from `core/src/render.rs`). Callers of the core never see it.

**2026-11-13, from the strict review of tickets 22-24.** It found eight things; six are done, two are left open.

Done after the review:

- The import cycle between `documents` and `render` is gone: `render_document` and `DocumentToShow` moved into the documents module, and `render.rs` only reads and renders.
- "The window the request came from shows it" is decided once, in `claim`: `Given::AlreadyHere` and `Given::AlreadyElsewhere`, and `Shown` likewise, replace `AlreadyOpen`, and `Target::from` is gone, since `claim` matches on `Target` itself. That also settles the naming finding: `Target::from` read like `From::from`, and the struct variant `AlreadyOpen { window }` sat beside tuple variants, where the new variants are tuple variants too.
- The page's fake app (`answerAsTheApp` and `mockTheApp` in `page/src/test-support.ts`) is typed with the page's `DocumentToShow`, so it can't drift from the answer the page reads.
- Tests for a `render_document` call that itself fails.
- The documents tests are split by feature, in `app/src/documents/tests/`: `give.rs`, `handover.rs` and `watch.rs`, with the shared helpers in `mod.rs`.

Left open:

- (6) `give` fails halfway when the page can't be told it shows another document: the table and the watch already have the new document, while the window keeps the old title and page, so `document_path` and relative links use the new document's folder. It is rare (a live webview that fails to run an event), and `document_given_to_a_window_whose_page_cannot_be_told_reports_the_failure` checks only that it fails. The remedy, if taken up: `claim` returns the entry it replaced, and `give` restores it when the send fails.
- (7) Tests that prove less than they could. `wait_for_render_ahead` and `opening_an_open_document_again_starts_no_render_for_its_window` still read the table's internals, since nothing public shows a render ahead finish. `watch_that_cannot_start_is_noticed_once_the_page_asks` waits 500 ms and passes whether the notice was held or sent at once, so it proves less than its name (the hold itself is covered by `notice_held_before_the_page_asks_is_sent_when_it_first_asks`). Nothing tests `Documents::reload` when its send fails.
