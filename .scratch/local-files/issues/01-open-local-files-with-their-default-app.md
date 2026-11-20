# 01: Opening non-Markdown local files with their default app, opt-in

**What to build:** When the reader switches it on, a link to a local file that isn't a **Document** (a PDF, an image, a spreadsheet) opens it in the program Windows opens that kind of file with, instead of revealing it in File Explorer as the **Link action** does today.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later ("opening non-Markdown local files with their default app, opt-in").

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Safety: link targets are untrusted. Which kinds never open this way (executables, scripts, shortcuts, installers): an allowlist of kinds, or a denylist as mdhero has? Does Downlight ask first, naming the file?
  - **Network share** rules hold: a link to a file on one asks first, as following a link there already does.
  - The core's link resolution returns a new link action; the app's per-OS module opens it.
  - The switch: in the **Settings**, and off by default.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). mdhero opens some kinds with the default app behind an allowlist and a denylist (`.scratch/mdhero-comparison/features.md`). Not yet grilled.
