# downlight

A small, lightweight Markdown viewer.

v1 ships for Windows x64 only, and CI runs on Windows. Other OSes get only the plain fallbacks in `app/src/platform/` until their ports.

## Coding standards

Writing or reviewing code follows these standards and the glossary's terms:

@CODING_STANDARDS.md
@GLOSSARY.md

## Commands

- `npm run check` is the check command: formatting (Prettier, rustfmt), lints (ESLint, Clippy), the type check, cargo-deny and all tests. A change is done when it passes. CI runs it on every push and pull request. It needs Node, Rust with rustfmt and Clippy, and cargo-deny (`cargo install --locked cargo-deny`), plus `npm install` once.
- `npm test` runs all tests: `npm run test:rust` is `cargo test` for the core and the app, and `npm run test:page` runs the page's tests (Vitest with happy-dom, `page/src/*.test.ts`). `npx vitest` reruns the page's tests on every save.
- The rendered fixtures: each document in `core/tests/fixtures/` is committed next to the HTML the core renders for it, and the page's tests load that HTML. A core test fails when rendering no longer gives the committed HTML. After an intended change to rendering, `npm run fixtures` writes the HTML again; review its diff before committing.
- The contract samples: every command and message between the page and the app is typed in `page/src/app.ts`, the only page module that calls `invoke` or `listen` (ESLint enforces it); the tests' fake app (`fakeTheApp` in `page/src/test-support.ts`) answers in the same types. The app's tests write a sample of each answer and message it sends, and of the menu entries and timing reports it takes, to `app/tests/fixtures/` (`app/src/contract.rs`), and a test fails when they no longer match. The page's tests read them (`page/src/contract.test.ts`): they drive each answer and message through the page, and check the keys the page sends against the samples of what the app takes. After an intended change to a shape, `npm run fixtures` writes them again too.
- `npm run tauri dev` runs the app against Vite's dev server; `npm run tauri build` builds `target/release/downlight.exe` and the per-user installer `target/release/bundle/nsis/Downlight_<version>_x64-setup.exe` (its first run downloads NSIS).
- After changing a capability in `app/capabilities/`, run `cargo clean -p tauri --release` before `npm run tauri build`. With `removeUnusedCommands`, the commands to keep are baked into the `tauri` crate's build output, and Cargo doesn't rebuild it when a capability changes, so a command the capability now allows stays compiled out ("Command listen not found" in the window).
- The timing switch: with `DOWNLIGHT_TIMING` set, the app writes `timing ...` lines to stderr for first paint, its stages and every page task or frame over 100 ms. `npm run documents` writes the test documents (100 KB and 1 MB) to `target/documents/`; `npm run measure` runs the release build on them and prints medians of 5 runs. Ticket 02's Results hold the baseline.
- The Cargo packages are named differently from their folders: `core/` is `downlight-core` and `app/` is `downlight`. Run one crate's tests with `cargo test -p downlight-core`.

## Commits

Commit messages follow Conventional Commits and are a single short subject line, for example `feat(find): highlight matches as you type`.

## Agent skills

### Issue tracker

Issues and specs live as local Markdown files under `.scratch/<feature-slug>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

The five default triage roles, recorded as a `Status:` line in each issue file. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
