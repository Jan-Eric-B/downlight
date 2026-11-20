# 01: About Downlight

**What to build:** The reader can see Downlight's version, its copyright and license (GPL-3.0, with no warranty), and reach the full license and the third-party notices, from inside the app.

Spec: none yet. Grill it first (`/grill-with-docs`).

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - GPL-3.0 section 5(d) asks a program with an interactive user interface to display "Appropriate Legal Notices" (copyright, no warranty, that it is GPL-licensed and how to see the license). Today only `LICENSE.txt` and `THIRD-PARTY-NOTICES.txt` beside the exe carry them. What has to show in the app to meet it?
  - Where: a section at the bottom of the settings flyout, an entry in the right-click menu, or a small window of its own?
  - How the license and notices open: in Downlight itself as text documents (they are text files beside the exe), or in the **Text file program**?
  - What else it shows: the WebView2 version, a link to the source (which makes no request until clicked, and then goes to the browser).

## Comments

**2026-11-20, from the repo owner.** Picked from the mdhero comparison (`.scratch/mdhero-comparison/features.md`, "About dialog"). The GPL point came up while explaining it.
