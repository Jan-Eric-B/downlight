# 27: One page module speaks to the app

**What to build:** A refactor of the page, with no change the reader sees. Every command the page invokes and every message it listens for is named and typed in one page module, the page's counterpart to `app/src/message.rs`. Tauri's IPC is its adapter in the app, and the tests' fake app is the other. The app's tests write a sample of each answer and message as JSON, and the page's tests load them, so the two sides can't drift apart.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** ready-for-agent

- [ ] One module in `page/src/` names every command (`render_document`, `follow_link`, `open_document`, `show_document_menu`, `close_window`, `show_window`, the timing reports, `reload_document` until ticket 26 removes it) with typed arguments and answers, and every message (`document-shown`, `document-changed`, `scroll-to-heading`, `notice`, `menu-command`) with its typed payload. No other page module calls `invoke` or `listen`.
- [ ] `Place` and `DocumentToShow` (`view.ts`), `Followed` (`links.ts`), the menu entries (`commands.ts`) and the timing marks (`timing.ts`) move there.
- [ ] Contract samples, in the style of the rendered fixtures: a Rust test writes each answer and message the app sends as JSON under a fixtures folder, checks the committed files as `core/tests/rendered_fixtures.rs` does, and the page's tests read them through the module's types. A change to a shape fails a test on the side that didn't change.
- [ ] The fake app in `page/src/test-support.ts` answers through the same module, so the tests can't call a command the app doesn't have.
- [ ] No new dependency.

## Comments

**2026-11-13, from the architecture review.** Today command names are string literals in six page files, and event names in two; `listenToApp(event: string, handler: (payload: string) => void)` takes any string. `Place`, `DocumentToShow`, `Followed`, the menu entries and the timing marks are written out by hand in both languages. The Rust tests pin the JSON shapes (`handover.rs`, `links.rs`, `message.rs`), but nothing ties them to the TypeScript types. Tickets 12, 15 and 16 each add commands or messages, so this pays back at once. `answerAsTheApp` re-implements `Documents::handover`'s decision in TypeScript; keep that, but make it answer in the module's types.
