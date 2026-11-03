# 01: Walking skeleton

**What to build:** Running `downlight <file>` from the build output opens a window with the native title bar that shows the Markdown file rendered by the core with comrak and styled with github-markdown-css, on Mica on Windows 11. This is the first path through every layer: the core crate renders, the app crate reads the command line and opens the window, and the page shows the HTML. Nothing else yet.

Spec: [Downlight v1](../spec.md)

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] A Cargo workspace with the core crate (no Tauri dependency) and the app crate, on the latest stable Tauri 2.x (not the 3.0 alpha). Each dependency's version is taken from the registry when it is added.
- [ ] The page is plain TypeScript built with Vite, with no UI framework; github-markdown-css (its MIT notice kept) in light or dark, following Windows.
- [ ] The core has one render entry point: a file's bytes and path in, HTML out. At least one `cargo test` passes against it.
- [ ] `downlight <file>` with a Markdown file shows the rendered document.
- [ ] Rendering runs off the app's main thread.
- [ ] On Windows 11 the window is transparent with the Mica window effect, and the page draws no background of its own.
- [ ] The content security policy allows Downlight's own scripts and nothing else.
- [ ] It builds and runs on Windows x64 with the system WebView2; no WebView2 runtime is bundled or downloaded.

Tooling and checks (the tools `CODING_STANDARDS.md` relies on):

- [ ] rustfmt with its default style.
- [ ] Clippy's default lints fail the check, and workspace lints add `unwrap_used`, `expect_used`, `panic`, `todo`, `dbg_macro` and `undocumented_unsafe_blocks`. The first five are allowed in tests. `unsafe_code` is forbidden in the core crate.
- [ ] cargo-deny checks licenses against a GPL-3.0-compatible allowlist, and checks security advisories.
- [ ] TypeScript in strict mode; ESLint with typescript-eslint's strict type-checked rules; Prettier for formatting. Versions come from the registry.
- [ ] An `.editorconfig` (UTF-8, LF, final newline) and a `.gitattributes` with `* text=auto eol=lf`.
- [ ] `.gitignore` covers `node_modules/`, Vite's `dist/` and Tauri's generated schemas, and anchors `/target/` and `/debug/` to the root.
- [ ] One check command runs the formatting checks, the lints, the type check, cargo-deny and all tests. It is named in `CLAUDE.md`.
- [ ] A GitHub Actions workflow on Windows runs the check command on every push and pull request.
