# 03: Page test harness

**What to build:** The page's own TypeScript can be tested with Vitest and happy-dom against HTML that the core really produces. The core's tests render a small set of fixture documents and check them against committed rendered fixtures; the page's tests load the same rendered fixtures, so both sides agree on the HTML's shape.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 02 (Performance spike and stack decision)

**Status:** ready-for-agent

- [ ] Vitest with happy-dom runs the page tests with one command.
- [ ] A core test renders the fixture documents and fails when its output differs from the committed rendered fixtures. There is a documented way to update them.
- [ ] One page test passes against a rendered fixture.
- [ ] The commands for the core tests and the page tests are written down where contributors look.

## Comments

**2026-11-03, review of ticket 08.** Ctrl+W (`page/src/window.ts`) goes by the letter the key types on a layout that types Latin letters and by the key's place otherwise. That branch has no test yet; cover both cases once the page tests run.

**2026-11-03, architecture review.** The page calls the app with `invoke` directly in `page/src/main.ts`, `page/src/window.ts` and `page/src/timing.ts`, with the command names as bare strings that have to match `generate_handler!` in `app/src/main.rs` and the result types asserted at each call. The page tests here and in later tickets (document view, reload, ticks, Find, Ctrl+W) need to stand in for the app. Before choosing how, check whether Tauri's own IPC mocking (`@tauri-apps/api/mocks`, at the version in `package-lock.json`) covers it. If it does, a page module of its own for the app's commands is only tidiness; if it doesn't, add `page/src/app.ts` with one typed function per command (`renderDocument()`, `closeWindow()`, the timing reports), so the tests replace one module and the command names live in one place. Expected size: about 40 lines across four files.
