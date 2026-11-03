# downlight

A small, lightweight Markdown viewer.

v1 ships for Windows x64 only, and CI runs on Windows. Other OSes get only the plain fallbacks in `app/src/platform/` until their ports.

## Coding standards

Writing or reviewing code follows these standards and the glossary's terms:

@CODING_STANDARDS.md
@GLOSSARY.md

## Commands

- `npm run check` is the check command: formatting (Prettier, rustfmt), lints (ESLint, Clippy), the type check, cargo-deny and all tests. A change is done when it passes. CI runs it on every push and pull request. It needs Node, Rust with rustfmt and Clippy, and cargo-deny (`cargo install --locked cargo-deny`), plus `npm install` once.
- `npm run tauri dev` runs the app against Vite's dev server; `npm run tauri build` builds `target/release/downlight.exe`.
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
