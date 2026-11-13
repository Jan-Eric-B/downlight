# 34: Small decisions left from the architecture review

**What to build:** Settle a few small behaviors that need a decision before they can be built.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** needs-triage

- [ ] **Files that aren't Markdown from the command line.** `downlight notes.txt` or `downlight some.png` renders the file as Markdown today (the spec's Open with for other formats is "later"). Decide: show it as Markdown anyway, show it as plain text, or say Downlight opens Markdown files, as a drop does.
- [ ] **Drive-relative paths in a second launch.** `downlight C:notes.md` in a second launch resolves against the running process's current folder on that drive, not the second launch's. Decide whether to resolve it in the second launch before handing it over, or say it can't be opened.
- [ ] **Dropping a shortcut.** Dropping a `.lnk` to a Markdown file says "Downlight opens Markdown files". Decide whether to follow the shortcut (through the shell, never running it) or name shortcuts in the message.
- [ ] **Line numbers in raw HTML.** A document's raw HTML may carry its own `data-sourcepos`, giving its blocks false line numbers that reload anchoring and Find's first match then use. Decide whether to drop the attribute from raw HTML (in the same pass that renames filtered tags in `core/src/sanitize.rs`) or accept it as the document's own doing.
- [ ] **Mapped drives and shares.** A document on a mapped drive (`Z:`) keeps its relative local images, while the same document opened by its UNC path loses them (images on shares never load). Decide whether that difference is fine or whether drive letters on network drives count as shares too.

## Comments

**2026-11-13, from the architecture review.** Each is low severity. Once decided, each is a small task with a test first.
