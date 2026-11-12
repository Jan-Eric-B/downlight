# 22: One module owns a window's shown document

**What to build:** A refactor of the app, with no change the reader sees. One module owns everything about the document a window shows: giving it to the window, watching its file, rendering ahead, holding notices and the heading until the page asks, reloading it only while the window still shows it, and forgetting it. App-to-page messages go through one typed adapter, so this lifecycle can be tested without a webview.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 21 (A save never reloads a document the window has just left)

**Status:** needs-triage

- [ ] One module's small interface (roughly: give, reload, hand over to the page, forget) replaces the eight `OpenWindows` methods that callers chain today with a (label, key) pair, and `app/src/reload.rs` folds into it.
- [ ] Every message the app sends a page (`document-shown`, `document-changed` with `reload` or `gone`, `scroll-to-heading`, `notice`, `menu-command`) goes through one typed set of messages, with Tauri's `emit_to` as the app's adapter and a recorder as the tests' adapter.
- [ ] Tests drive the lifecycle through that interface and check the messages sent: a document given and then changed, a reload overtaken by another document, a notice held until the page asks, a window that closes while its render is running.

## Comments

**2026-11-12, from ticket 14's architecture review.** Why this is worth doing, and other leads from the same review.

The problem: per-window state lives in `ShownDocument` in `app/src/window.rs` (render ahead, watch, `page_asked`, held notice, heading), and `window.rs`, `reload.rs` and `render.rs` each drive part of it. To follow "a window is given a document" you go through `show`, `claim`, `ShownDocument::given`, `render::start`, `reload::watch_document`, `Watch::start`, `keep_watch` or `hold_notice`, and `notice::show_notice`. `window.rs` and `reload.rs` import each other. `reload.rs` is 68 lines of glue: deleting it would move its code, not remove any. Ticket 21's race comes from this shape, two calls where one was needed. Because the commands and `show` take a concrete `AppHandle`, the orchestration (`show`, all of `reload.rs`, the notice ordering in `render_document`, `links::show_document`, `open::show_when_dropped`) has no tests; `app/src/window/tests.rs` tests `claim` directly and polls the table's internals to see a render finish.

Related: `render_document` can't tell a show from a reload. The heading goes out with every render and the page drops it on a reload, so the app and the page each hold half of that protocol. `page_asked` with `held_notice` and `page_asked` with `heading` are two copies of the same "hold it until the page asks" step. The typed messages would make both visible in one place.

Other leads, smaller and not part of this ticket unless it touches them:

- `window.rs` also builds the Tauri window (navigation guard, backdrop, the 5 s fallback for showing it, the key handler), holds the launch failures and their report, and formats titles; its tests mix titles and failure reports with the table's logic.
- `app/src/links.rs` tests its pure helpers (`destination`, `left_for_the_page`), but not `show_document`'s `window != label` check or its `scroll-to-heading` message.
- `OpenError`, `FollowError` and `LaunchFailure` carry `tauri::Error` from `show`, and `Claim::WindowGone` becomes `tauri::Error::WindowNotFound`, so the reader can see "window not found" where the window had just closed.
- Speculative, inside the core: sanitizing takes two calls that must run in order, `drop_filtered_tags` on the tree and then `sanitize` (`core/src/sanitize.rs`, called from `core/src/render.rs`). Callers of the core never see it.
