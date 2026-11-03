# 01: Walking skeleton

**What to build:** Running `downlight <file>` from the build output opens a window with the native title bar that shows the Markdown file rendered by the core with comrak and styled with github-markdown-css, on Mica on Windows 11. This is the first path through every layer: the core crate renders, the app crate reads the command line and opens the window, and the page shows the HTML. Nothing else yet.

Spec: [Downlight v1](../spec.md)

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [x] A Cargo workspace with the core crate (no Tauri dependency) and the app crate, on the latest stable Tauri 2.x (not the 3.0 alpha). Each dependency's version is taken from the registry when it is added.
- [x] The page is plain TypeScript built with Vite, with no UI framework; github-markdown-css (its MIT notice kept) in light or dark, following Windows.
- [x] The core has one render entry point: a file's bytes and path in, HTML out. At least one `cargo test` passes against it.
- [x] `downlight <file>` with a Markdown file shows the rendered document.
- [x] Rendering runs off the app's main thread.
- [x] On Windows 11 the window is transparent with the Mica window effect, and the page draws no background of its own.
- [x] The content security policy allows Downlight's own scripts and nothing else.
- [x] It builds and runs on Windows x64 with the system WebView2; no WebView2 runtime is bundled or downloaded.

Tooling and checks (the tools `CODING_STANDARDS.md` relies on):

- [x] rustfmt with its default style.
- [x] Clippy's default lints fail the check, and workspace lints add `unwrap_used`, `expect_used`, `panic`, `todo`, `dbg_macro` and `undocumented_unsafe_blocks`. `unwrap_used`, `expect_used`, `panic` and `dbg_macro` are allowed in tests; `todo` is not, since a `todo!` left in a test means the test isn't finished. `unsafe_code` is forbidden in the core crate.
- [x] cargo-deny checks licenses against a GPL-3.0-compatible allowlist, and checks security advisories.
- [x] TypeScript in strict mode; ESLint with typescript-eslint's strict type-checked rules; Prettier for formatting. Versions come from the registry.
- [x] An `.editorconfig` (UTF-8, LF, final newline) and a `.gitattributes` with `* text=auto eol=lf`.
- [x] `.gitignore` covers `node_modules/`, Vite's `dist/` and Tauri's generated schemas, and anchors `/target/` and `/debug/` to the root.
- [x] One check command runs the formatting checks, the lints, the type check, cargo-deny and all tests. It is named in `CLAUDE.md`.
- [ ] A GitHub Actions workflow on Windows runs the check command on every push and pull request.

## Comments

**2026-11-03, implementation.** Layout: `core/` (downlight-core), `app/` (downlight, Tauri 2.12.5), `page/` (Vite). `npm run check` passes locally. `npm run tauri build` gives `target/release/downlight.exe`, which showed a sample document on Mica in dark mode. Over WebView2's debug port, the served CSP blocked inline scripts, remote scripts, `eval` and remote images. A missing file and a missing argument show their message in the window.

- TypeScript is 6.0.3, not 7.0: typescript-eslint 8.72 supports `<6.1.0`. `~6.0.3` keeps it there.
- Mica uses Tauri's own `Effect::Mica` (window-vibrancy inside Tauri), so there's no extra crate and no `unsafe`. On Windows 10 the window stays transparent until ticket 04 adds the solid background.
- comrak has no default features (no CLI, no Oniguruma). Its safe mode leaves raw HTML out until ticket 06 adds sanitizing.
- github-markdown-css's MIT notice ships in `page/dist/licenses.md`, written by Vite's `build.license`.
- `.npmrc` pins the public npm registry, so `package-lock.json` resolves on GitHub rather than through a local mirror. Prettier skips `*.md`.
- Open: the workflow is written but hasn't run on GitHub yet; its box gets ticked after the first green run. The Cargo manifests have no `license` field (GPL-3.0-only or -or-later is undecided). `npm audit` reports a moderate minimatch advisory in ESLint's dev tooling with no in-range fix yet. `cargo audit` warns about two Linux-only GTK crates, which matter for the Linux port.
