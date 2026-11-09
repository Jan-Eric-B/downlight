# 03: Page test harness

**What to build:** The page's own TypeScript can be tested with Vitest and happy-dom against HTML that the core really produces. The core's tests render a small set of fixture documents and check them against committed rendered fixtures; the page's tests load the same rendered fixtures, so both sides agree on the HTML's shape.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 02 (Performance spike and stack decision)

**Status:** ready-for-agent

- [x] Vitest with happy-dom runs the page tests with one command.
- [x] A core test renders the fixture documents and fails when its output differs from the committed rendered fixtures. There is a documented way to update them.
- [x] One page test passes against a rendered fixture.
- [x] The commands for the core tests and the page tests are written down where contributors look.

## Comments

**2026-11-03, review of ticket 08.** Ctrl+W (`page/src/window.ts`) goes by the letter the key types on a layout that types Latin letters and by the key's place otherwise. That branch has no test yet; cover both cases once the page tests run.

**2026-11-03, architecture review.** The page calls the app with `invoke` directly in `page/src/main.ts`, `page/src/window.ts` and `page/src/timing.ts`, with the command names as bare strings that have to match `generate_handler!` in `app/src/main.rs` and the result types asserted at each call. The page tests here and in later tickets (document view, reload, ticks, Find, Ctrl+W) need to stand in for the app. Before choosing how, check whether Tauri's own IPC mocking (`@tauri-apps/api/mocks`, at the version in `package-lock.json`) covers it. If it does, a page module of its own for the app's commands is only tidiness; if it doesn't, add `page/src/app.ts` with one typed function per command (`renderDocument()`, `closeWindow()`, the timing reports), so the tests replace one module and the command names live in one place. Expected size: about 40 lines across four files.

**2026-11-09, implementation.** Vitest 5.1.0 and happy-dom 21.0.4 run the page's tests (`npm run test:page`), configured in `vite.config.ts`. `npm test`, and so `npm run check` and CI, now runs `npm run test:rust` and then `npm run test:page`. The commands are in `CLAUDE.md`.

- **The app in page tests:** `mockIPC` from `@tauri-apps/api/mocks` (2.12.4) replaces `invoke` and calls its handler synchronously, so a test answers the page's commands without a webview. There is no `page/src/app.ts`; the command names stay bare strings in the page and the tests.
- **Rendered fixtures:** `core/tests/fixtures/<name>.md` is committed next to `<name>.html`. `core/tests/rendered_fixtures.rs` renders each document with only its file name as the path, so the HTML doesn't depend on where the repository is checked out, and fails when it differs from the committed HTML. `npm run fixtures` runs the file's ignored test that writes the HTML again; it needs no environment variable, so it works the same in every shell. Prettier skips the folder. Page tests import the HTML with Vite's `?raw`.
- **The one fixture,** `readme.md`, is a small README. Its table and task list render as plain paragraphs until ticket 04 turns on GFM; that ticket's `npm run fixtures` diff shows them appear. Tickets 10, 13 and 14 add the fixtures they need.
- **Page tests:** `page/src/main.test.ts` loads the page with the app answering `render_document` with `readme.html` and checks the document is shown. `page/src/window.test.ts` covers Ctrl+W: the letter the key types on Latin layouts (QWERTY, AZERTY in both directions), the key's place on Cyrillic, no Ctrl, and Shift, Alt or Meta held too. Each test was seen failing with the code it covers broken.
- `npm audit` still reports only ticket 01's minimatch advisory, now counted as 10 entries along ESLint's and typescript-eslint's dependency chain; Vitest and happy-dom added none.
