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
