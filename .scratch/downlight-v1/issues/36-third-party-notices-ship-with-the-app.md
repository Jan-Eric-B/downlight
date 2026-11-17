# 36: Third-party notices ship with the app

**What to build:** Every license notice the app's dependencies require in a binary distribution ships with Downlight, and so does Downlight's own `LICENSE` (GPL-3.0). Today only the page's notices ship (`licenses.md` from Vite's `build.license`, and `grammar-licenses.md` from ticket 17), embedded in the exe where no reader can see them; the Rust crates' MIT and Apache-2.0 notices (Tauri, comrak, ammonia, two-face and the rest, 274 packages for `x86_64-pc-windows-msvc`) and Downlight's `LICENSE` ship nowhere, though MIT and Apache-2.0 ask for their notices in every copy.

Downlight ships only as the per-user NSIS installer (spec: Platforms), so every copy gets the files the installer writes. The notices go there as readable files next to `downlight.exe`, and leave the exe.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 17 (Installer and file associations)

**Status:** ready-for-agent

- [ ] The notices of every Rust crate in the release build are generated from `Cargo.lock` by cargo-about, for the same target as `deny.toml` (`x86_64-pc-windows-msvc`) and accepting only the licenses `deny.toml` allows, not written by hand. The workspace's own crates are left out, as in cargo-deny.
- [ ] One third-party notices file holds them all: the Rust crates, the page's packages (what Vite's `build.license` writes today, and Fluent UI System Icons, which ships no license file of its own) and the syntax grammars (two-face's listing). It is committed, so its diff is reviewed.
- [ ] The installer puts that file and Downlight's `LICENSE` in the install folder next to `downlight.exe`, and the uninstaller removes them.
- [ ] The notices no longer ship inside the exe: `licenses.md` and `grammar-licenses.md` leave `page/dist/`. The Fluent icons banner at the top of the page's bundle stays, since it is the notice inside the code it covers.
- [ ] No test of one part writes into another part's folder: `core/tests/grammar_licenses.rs` stops writing `page/public/grammar-licenses.md`. The core keeps checking that every grammar license is compatible with GPL-3.0; whatever assembles the notices file lives with the app, which owns the installer.
- [ ] The check command fails when the committed notices file no longer matches what cargo-about, Vite and two-face give for the current lockfiles, naming `npm run fixtures` as the fix.
- [ ] `npm run fixtures` writes the notices file again after a dependency changes.
- [ ] CI installs cargo-about (`cargo install --locked cargo-about`, as it does cargo-deny), and CLAUDE.md's Commands name it with the check command's other tools.
- [ ] Hand check: after installing, the install folder holds `LICENSE` and the notices file, both open as text, and uninstalling removes them.

## Comments

**2026-11-16, from ticket 17.** Found while listing two-face's grammar licenses. A new tool (cargo-about or similar) needs the owner's say, so it was left out of ticket 17. To decide in triage: embed the notices in the exe as the page's are (ticket 01's precedent, about 190 KB for the grammars alone) or install them as readable files next to the exe.

**2026-11-16, from the review of tickets 16, 17 and 20.** `core/tests/grammar_licenses.rs` writes `page/public/grammar-licenses.md`, a core test reaching into the page's folder against "three parts, one direction". Where the notices are generated and placed is part of this ticket: keep the knowledge in the core, but have the build or the app place the file, or move the test to `app/tests`.

**2026-11-17, triage, decided by the repo owner.**

- **cargo-about is added.** cargo-deny lists license IDs but can't write license texts, and a hand-written reader of `~/.cargo/registry` would be more code to keep. cargo-about comes from Embark, who also make cargo-deny, and is under MIT or Apache-2.0.
- **Installed files, not embedded.** Downlight has no portable build, so the installer reaches every copy, and a file in the install folder is one the reader can open. Nothing in the app shows the embedded notices today, and the exe gets about 190 KB smaller without them. If a portable build comes later, it ships the same two files beside the exe.
- Left to the implementer: the file's name and format (one Markdown or text file), how the installer gets it (Tauri's bundle resources or `app/installer/hooks.nsh`; check Tauri's current docs), and how Vite's license output and two-face's listing are combined with cargo-about's.
