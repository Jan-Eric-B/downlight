# 26: One message makes the page ask, and every request names its showing

**What to build:** A refactor of the app-to-page protocol, which also closes three narrow races. F5 asks for the document directly instead of going through the app and back. The app's two messages that only mean "ask now" become one. A `gone`, a held notice and a followed link each carry, or are checked against, the showing they are about, so a stale one is told apart by data.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 25 (One pipeline places every document in the page's view), 27 (One page module speaks to the app)

**Status:** ready-for-human

- [x] F5, Ctrl+R and the right-click menu's Reload call the view's request (ticket 25) with the showing the view shows; `reload_document` and `Documents::reload` are gone, with their tests. `Documents::handover` already starts a new read when no render is ahead (`next_render_after_the_first_reads_the_file_again`).
- [x] `document-shown` and `document-changed: "reload"`, which since ticket 23 both only make the page ask, are one message.
- [x] `gone` carries the showing it is about, and the page shows the bar only for the showing its view shows. This closes the trade-off ticket 23 recorded: a stale `gone` no longer needs the bar cleared on every show, and a `gone` for the new document is no longer dropped.
- [x] A notice held until the page asks goes in the `render_document` answer rather than as an event. `plugin:event|listen` is an async command like `render_document`, so the page's listener may not be registered when `handover` sends it today, and an event with no listener is dropped.
- [x] `follow_link` takes the showing whose document the link was in, and a link from a showing the window has left does nothing. Today a link clicked in the old document while the next one renders (TypeScript's grammar alone takes about 300 ms on first use) resolves against the new document's folder and opens the wrong file or says "Can't find".
- [x] Timing: a hand reload's stages are reported under the place the page gets (`reload`), not under the label the render started with.
- [x] Tested first: in `app/src/documents/tests/` with the recorder for the app's half, and in the page's tests for a stale `gone`, a held notice in the answer and a stale link.

## Results

Built 2026-11-16. `npm run check` passes.

- **One message.** `Message::DocumentShown` and `Message::Reload` are `Message::AskForDocument`, sent as `ask-for-document` with no payload. `gone` is `Message::Gone(Showing)`, its own event `document-gone` with the showing as a number, so `document-changed` is gone. The page's view keeps the showing of the last `gone` (`goneShowing` in `page/src/view.ts`) and shows the bar only while the view shows that showing; a request still clears it, since it reads the file again. The clearing on every show is gone.
- **F5.** The command table's `reload` calls `requestDocument()`; `reload_document`, `Documents::reload` and their two tests are gone, and with them ticket 22's open item 7 about `Documents::reload`'s failed send.
- **Held notice.** `Handover` and `DocumentToShow` carry `notice` (`Notice | null` on the page). `handover` no longer sends anything, so it now takes everything under one lock. The page shows the notice as soon as the answer comes, even if a later request overtakes it, since the app hands it over once.
- **Links.** `follow_link` takes `showing`; `follow` looks the document up with the new `Documents::document_path_shown_as(window, showing)`, which gives None once the window has left that showing, so the link does nothing (`Followed::Nothing`). The page sends `showingInView()` (`page/src/view.ts`), and sends nothing while the view shows no render.
- **Timing.** `Stages` and `render::start` no longer carry a label; `Timing::report` takes it, and `render_document` reports by `Place::reported_as()` (`read-and-render` for a show, `reload` for a reload), the names `scripts/measure.js` reads.

**Beyond the plan:**

- The contract samples: `messages.json` has `askForDocument` in place of `documentShown` and `reload`, and `gone` with the samples' showing (`Showing::sample()`, 7, the same as `render_document.json`'s); `render_document.json` has `notice: null` in every answer and a new `showWithHeldNotice`. A contract test checks that a link names the showing of the sampled answer.
- Tests changed because the shapes changed: the page's tests send `ask-for-document` and `document-gone` through new helpers in `page/src/test-support.ts` (`saveTheFile`, `deleteTheFile`, and `giveAnotherDocument` now returns the showing the window left); the stale `gone` test in `view.test.ts` names the old showing; `links.test.ts` shows its documents through the real document view, so a link has a showing, and its expected `follow_link` arguments include it; the Reload tests in `commands.test.ts` start the view and check the `render_document` requests, and "a reload the app can't start shows why" became "a reload the app can't answer keeps the document and shows why". In the app's tests, `watch_that_cannot_start_is_noticed_once_the_page_asks` takes the notice from the first answer, or from a message if the watch failed only after the page asked.
- New tests: `request_by_hand_reads_the_file_again_as_a_reload_and_tells_the_page_nothing`, `render_is_timed_as_the_place_the_page_gets`, `gone_names_the_showing_of_the_document_the_window_was_given_since`, `notice_held_before_the_page_asks_comes_with_its_first_answer`, `link_from_a_showing_the_window_has_left_has_no_document`; on the page, a stale `gone` after the new document is in view, a `gone` for the new document before it is in view, the held notice in the answer, and a link that names its showing while the next document renders.

**Not measured:** `npm run tauri build` couldn't replace `target/release/downlight.exe`, which was running. No rendering work changed, but a reload by hand now reads the file when the page asks rather than when the app is told, and the reports' labels moved; run `npm run measure` once to see that its `read-and-render` and `reload` lines still come.

**Not yet checked by hand** (release build): each changed command once in the app, since the contract samples don't pin parameter names (ticket 27's comment): F5, Ctrl+R and the menu's Reload reload at the reading position; a link to another document, a heading link and a Ctrl+click still work (`follow_link` now takes `showing`); deleting the shown file shows the bar, and restoring it reloads and hides it; a document whose file can't be watched (for example on a share that doesn't report changes) shows its "Can't watch" notice when the window opens.

## Comments

**2026-11-13, from the architecture review.** The unhandled rejection of `void invoke("reload_document")` was fixed separately in `page/src/commands.ts`; this ticket removes the command itself. Ticket 22's open item 7 ("nothing tests `Documents::reload` when its send fails") goes with it.

**2026-11-13, from ticket 27.** The contract samples (`app/src/contract.rs`) pin the shapes of answers, messages and the menu and timing arguments, but not the names of a command's parameters: Tauri takes them from the Rust function. This ticket renames or adds some (`follow_link` takes a showing, `reload_document` goes), and a page-side name that no longer matches fails only at run time, so run each changed command once in the app. A new message variant, or a `gone` that carries its showing, gets its sample in `message.rs`'s `contract_samples` and `npm run fixtures`.
