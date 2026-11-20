# 01: Writing ticks to the file, opt-in

**What to build:** When the reader switches it on, a **Tick** on a task list item is written to the document's file (`- [ ]` becomes `- [x]`), so it is still there in other programs and after Downlight closes.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later ("writing ticks to the file, opt-in").

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - It contradicts the glossary twice: Downlight is "reading only; changing a file happens in another program", and a Tick "is never written to the file". Is this the one exception, and how do those entries change?
  - Safety: write only the one character at the item's line, and only if the file is unchanged since it was shown; what if it changed (a **Reload** came, or is coming)?
  - Encodings and line endings kept exactly (UTF-8, Windows-1252, byte order marks), and files Downlight may not write (read-only, a **Network share**).
  - The save triggers Downlight's own watch: the **Reload** that follows keeps the ticks and the **Reading position**.
  - The switch: in the **Settings**, and off by default.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). Not yet grilled.
