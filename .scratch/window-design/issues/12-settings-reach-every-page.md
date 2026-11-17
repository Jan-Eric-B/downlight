# 12: Settings reach every page

**What to build:** A refactor before the first live setting, with no change the reader sees. The app hands every page one snapshot of the settings: in the page's URL when its window opens, and as one `settings` message to every window whenever a setting changes. The page hears it in one place, so Full width, zoom, theme, the backdrop and the accent color each become a field and a listener rather than a new message, URL flag, contract sample and fake-app answer of their own.

Spec: [Window design](../spec.md)

**Blocked by:** v1 [15](../../downlight-v1/issues/15-edit-and-choose-editor.md) (Edit and Choose editor), which added `app/src/settings.rs`

**Status:** ready-for-agent

- [ ] One `PageSettings` snapshot (the settings a page shows; the editor stays in the app) is typed in `page/src/app.ts`, with a contract sample (`npm run fixtures`).
- [ ] The page gets its first snapshot in its URL, as ticket 16's comments already plan for zoom, so first paint waits for no round trip.
- [ ] `SettingsStore` gives the whole snapshot (`get() -> Settings`) instead of one accessor per field, and `update` returns the new snapshot so its caller can send it.
- [ ] `Documents::send_to_every(message)` sends a message to every window, through `Pages`, so the recorder sees the broadcast; no second path through `AppHandle::emit`.
- [ ] The page's `whenSettingsChange(listener)` runs the listener once at start and on every change. `backdrop.ts` moves onto it instead of reading `pageFlags().backdrop` once.
- [ ] Tests: the snapshot in the URL and in the message match the contract sample; a change reaches every window (recorder); the page's listener runs at start and on a message.

## Comments

**2026-11-16, from the design review of v1 tickets 12 to 15.** Each live setting would otherwise cost about 11 files: a message and its sample, a URL flag and its sample, the `Commands`/`Messages` types, the fake app, a listener, the command table and CSS. The backdrop is read once from the URL today, but ticket 08 changes it while the window is open, and nothing yet sends a message to every window. The repo owner chose the URL for the first snapshot (no round trip before first paint) over a command at start. Ticket 08's comment that prescribes per-field accessors on `SettingsStore` is replaced by the snapshot here. A per-window setting (theme on the window itself, ticket 08) goes on the `Windows` port the same way.

**2026-11-16, from v1 ticket 16 (Zoom and window size).** Zoom needed most of this, so it is built, the way this ticket describes it: `PageSettings` (`app/src/settings.rs`, only `zoom` so far) with `Settings::page()`; the first snapshot in the page's URL as `settings=<JSON>` (`page_url` in `app/src/window.rs`, sampled in `app/tests/fixtures/page_url.json`) and every change as one `settings` message (sampled in `messages.json`, the same snapshot); `SettingsStore::get()` instead of `editor()`, and `update` returning `Updated { settings, saved }`, writing nothing when the change changed nothing. The documents module owns the `SettingsStore` now (`Documents::new(pages, windows, settings)`), and `Documents::change_settings(change)` is the one way a setting changes: it saves, and sends the snapshot to every window through `Documents::send_to_every` (so through `Pages`, which the recorder sees) when what pages show changed. Edit and Choose editor... go through it too. In the page, `page/src/settings.ts` has `startSettings()` (called first in `main.ts`) and `whenSettingsChange(listener)`, which runs at once and on each message; `firstSettings()` in `app.ts` reads the URL. Tests: `settings.test.ts`, the contract tests (URL and message give the same snapshot), and the recorder tests in `app/src/zoom.rs`. Not done: `backdrop.ts` still reads `pageFlags().backdrop` once, since the backdrop isn't a setting until ticket 08 makes it one; moving it onto `whenSettingsChange` then is left to that change. The checkboxes stay open for this ticket's own pass.
