# Coding standards

The rules for Downlight's code that no tool checks. Formatting, lints and strict types are enforced by the check command (rustfmt, Clippy, cargo-deny, ESLint with typescript-eslint, Prettier, TypeScript's strict mode), and a change is done when that command passes. Terms follow [GLOSSARY.md](GLOSSARY.md), and decisions live in [docs/adr/](docs/adr/).

## Structure

- **Three parts, one direction.** The core crate decodes, renders and resolves links; it reads and watches no document files, has no Tauri dependency and knows nothing about windows. The app crate owns the process, windows, menus, settings and everything that touches the OS: it reads and watches a document's file and passes its bytes to the core. The page shows what the core rendered and handles the reader's input. The page talks only to the app, and the app calls the core.
- **Folders by feature.** Name folders and modules after features (`render`, `links`, `watch`, `find`), and keep a feature's types, logic and tests together, so a feature is found by its name.
- **Deep modules.** Each module has a small interface that hides a lot: a caller of render passes bytes and a path and gets HTML back, never comrak options. A public item exists because something outside its module calls it.
- **One module per OS.** Code that calls OS APIs (editor launch, Mica, revealing a file) lives in the app's per-OS module behind one interface, and everything else stays OS-neutral. `unsafe` code lives only there.

## Untrusted documents

- Everything that comes from a document is untrusted: its HTML, link targets and image paths. HTML reaches the page only through the core's sanitizing, and a link reaches the OS only as an action returned by the core's link resolution.
- Each app command takes only what it needs (a path, a link target), validates it, and returns a typed result.
- Downlight makes no network requests of its own.

## Responsiveness

- The app's main thread stays free: file reads, rendering and watching run on background tasks, and their results come back as messages.
- Work that grows with a document's size (rendering, Find, anchoring a reload) is measured on the 1 MB test document with the timing switch when it is added or changed.

## Errors

- The core returns typed errors, one error type per feature. The app turns them into the messages the reader sees.
- Every error ends up handled or shown to the reader.

## Names and comments

- Names in code, tests and UI text use the glossary's terms (Document, Editor, Find, Tick, Reload). A new domain concept gets its glossary entry when it gets its first name in code.
- Rust names follow the Rust API Guidelines.
- Comments say why: a constraint, a reason, or the ticket or ADR behind a choice. When code needs a comment to say what it does, give it a clearer name instead.

## Tests

- Tests sit at the seams the spec names: the core's public interface and the app's watching (`cargo test`), and the page's document view and Find (Vitest with happy-dom). They drive the public interface, check what a caller can observe, and use the real comrak, ammonia and webview.
- A test's name states the behavior it checks, such as `reload_with_same_task_count_keeps_ticks`.
- Tests come first, from the ticket and the spec: decide what to check, write the tests and see them fail, then write the logic. A test written from the code mirrors its logic and passes whatever it does.

## Dependencies

- A dependency earns its place by saving real code or real risk. It is maintained, and its license is compatible with GPL-3.0.
- Versions come from the registry when the dependency is added (`cargo add`, `npm install`), and both lockfiles are committed.
