# 06: Filter a table's rows

**What to build:** The reader can show only a table's rows that match some text, to find their way in a large table.

Spec: none yet. Grill it first (`/grill-with-docs`). From the Sort grilling (2026-11-19, recorded in the [spec](../spec.md)), which put it on Later.

**Blocked by:** 01 to 05 (Sort a document's tables, [spec](../spec.md))

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - The Sort grilling rejected a filter on the table because it would be a second Find. Is it a Find option instead ("show matching rows only"), and does it then belong with Find's options (`.scratch/find-options/issues/01-find-options.md`)?
  - Like a **Sort**, it belongs to the showing and is never written to the file; does a **Reload** keep it?
  - What Find, copying, Edit and the counts do with hidden rows.

## Comments

**2026-11-20.** From the Sort grilling (2026-11-19): "Filtering rows (perhaps as a Find option, 'show matching rows only') ... go on Later."
