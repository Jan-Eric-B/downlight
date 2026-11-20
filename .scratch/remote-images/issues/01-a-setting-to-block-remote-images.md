# 01: A setting to block remote images

**What to build:** The reader can stop a document's remote images from loading, so opening a document never tells a server that it was read (tracking pixels, badges that log their requests).

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later ("before publishing").

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - The default: remote images load today, as on GitHub (v1 spec, story 27). Stay on by default, or off before publishing?
  - What a blocked image shows: its alt text, a placeholder, and a way to load them for this document once?
  - Where it is enforced: the page's content security policy, or the core leaving remote sources out, so nothing loads even before the page runs.
  - The switch in the **Settings** (the flyout's "View" group, or a privacy group), and whether a change reloads the shown document.
  - **Local images** and **Network share** rules stay as they are.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later), one of the two items to do before publishing. Not yet grilled.
