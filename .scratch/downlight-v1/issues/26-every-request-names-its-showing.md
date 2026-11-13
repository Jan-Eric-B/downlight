# 26: One message makes the page ask, and every request names its showing

**What to build:** A refactor of the app-to-page protocol, which also closes three narrow races. F5 asks for the document directly instead of going through the app and back. The app's two messages that only mean "ask now" become one. A `gone`, a held notice and a followed link each carry, or are checked against, the showing they are about, so a stale one is told apart by data.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 25 (One pipeline places every document in the page's view), 27 (One page module speaks to the app)

**Status:** ready-for-agent

- [ ] F5, Ctrl+R and the right-click menu's Reload call the view's request (ticket 25) with the showing the view shows; `reload_document` and `Documents::reload` are gone, with their tests. `Documents::handover` already starts a new read when no render is ahead (`next_render_after_the_first_reads_the_file_again`).
- [ ] `document-shown` and `document-changed: "reload"`, which since ticket 23 both only make the page ask, are one message.
- [ ] `gone` carries the showing it is about, and the page shows the bar only for the showing its view shows. This closes the trade-off ticket 23 recorded: a stale `gone` no longer needs the bar cleared on every show, and a `gone` for the new document is no longer dropped.
- [ ] A notice held until the page asks goes in the `render_document` answer rather than as an event. `plugin:event|listen` is an async command like `render_document`, so the page's listener may not be registered when `handover` sends it today, and an event with no listener is dropped.
- [ ] `follow_link` takes the showing whose document the link was in, and a link from a showing the window has left does nothing. Today a link clicked in the old document while the next one renders (TypeScript's grammar alone takes about 300 ms on first use) resolves against the new document's folder and opens the wrong file or says "Can't find".
- [ ] Timing: a hand reload's stages are reported under the place the page gets (`reload`), not under the label the render started with.
- [ ] Tested first: in `app/src/documents/tests/` with the recorder for the app's half, and in the page's tests for a stale `gone`, a held notice in the answer and a stale link.

## Comments

**2026-11-13, from the architecture review.** The unhandled rejection of `void invoke("reload_document")` was fixed separately in `page/src/commands.ts`; this ticket removes the command itself. Ticket 22's open item 7 ("nothing tests `Documents::reload` when its send fails") goes with it.

**2026-11-13, from ticket 27.** The contract samples (`app/src/contract.rs`) pin the shapes of answers, messages and the menu and timing arguments, but not the names of a command's parameters: Tauri takes them from the Rust function. This ticket renames or adds some (`follow_link` takes a showing, `reload_document` goes), and a page-side name that no longer matches fails only at run time, so run each changed command once in the app. A new message variant, or a `gone` that carries its showing, gets its sample in `message.rs`'s `contract_samples` and `npm run fixtures`.
