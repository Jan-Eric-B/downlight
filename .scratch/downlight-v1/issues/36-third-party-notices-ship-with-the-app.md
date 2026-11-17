# 36: Third-party notices ship with the app

**What to build:** Every license notice the app's dependencies require in a binary distribution ships with Downlight, and so does Downlight's own `LICENSE` (GPL-3.0). Today only the page's notices ship (`licenses.md` from Vite's `build.license`, and `grammar-licenses.md` from ticket 17); the Rust crates' MIT and Apache-2.0 notices (Tauri, comrak, ammonia, two-face and the rest) and Downlight's `LICENSE` ship nowhere, though MIT and Apache-2.0 ask for their notices in every copy.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 17 (Installer and file associations)

**Status:** needs-triage

- [ ] The notices of every Rust crate in the release build are generated from `Cargo.lock` (for example with cargo-about, which reads the licenses cargo-deny already checks), not written by hand.
- [ ] They ship with the app beside the page's notices, and the installer puts Downlight's `LICENSE` with the app.
- [ ] A test or the check command fails when a dependency is added whose notice isn't in the shipped file.
- [ ] `npm run fixtures` (or the build) writes the notices again after a dependency changes.

## Comments

**2026-11-16, from ticket 17.** Found while listing two-face's grammar licenses. A new tool (cargo-about or similar) needs the owner's say, so it was left out of ticket 17. To decide in triage: embed the notices in the exe as the page's are (ticket 01's precedent, about 190 KB for the grammars alone) or install them as readable files next to the exe.

**2026-11-16, from the review of tickets 16, 17 and 20.** `core/tests/grammar_licenses.rs` writes `page/public/grammar-licenses.md`, a core test reaching into the page's folder against "three parts, one direction". Where the notices are generated and placed is part of this ticket: keep the knowledge in the core, but have the build or the app place the file, or move the test to `app/tests`.
