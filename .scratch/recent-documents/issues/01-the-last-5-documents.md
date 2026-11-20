# 01: The last 5 documents in the empty window

**What to build:** The empty window lists the last 5 documents the reader opened, so one click shows one again. Nothing reopens by itself when Downlight starts.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, v1.x ("with no automatic reopening").

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - What counts as opened: from the command line, Open, a drop, a link to another document, Back and Forward?
  - Where the list is kept (beside the **Settings**, or in the same file), and how the reader clears it or removes one entry.
  - A listed file that is gone or now on a **Network share**: shown greyed, removed, or tried and failed with a **Notice**? Listing it must not touch the share's host.
  - What each entry shows: name, folder, when it was opened.
  - **The reading position across launches** (the owner wants it): a document opened again, after Downlight was closed, shows at the **Reading position** it had when the reader left it. For how many documents (mdhero keeps 500), kept where, and what if the file changed meanwhile (the line may be gone or moved)? Does it apply to every way of opening, or only from this list?

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (v1.x). mdhero's home screen lists recent files and remembers the reading position per file (`.scratch/mdhero-comparison/features.md`). Not yet grilled.
