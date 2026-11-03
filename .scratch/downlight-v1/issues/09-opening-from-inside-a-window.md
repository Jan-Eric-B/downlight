# 09: Opening from inside a window

**What to build:** From inside a window, Ctrl+O and dropping a file show a document there, and a window started without a file offers "Open file" and says that a file can be dropped. This ticket also creates the right-click menu on the document, which later tickets fill with their own commands.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 08 (One process, a window per document)

**Status:** ready-for-agent

- [ ] Ctrl+O shows an open dialog filtered to `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn` and `.mdwn`, and shows the chosen document in this window. If the document is open in another window, that window comes to the front instead.
- [ ] Dropping a Markdown file onto a window shows it there, by the same rule.
- [ ] Dropping anything else shows a short message saying that Downlight opens Markdown files.
- [ ] A window started without a file shows "Open file" and "or drop a file here".
- [ ] Right-clicking the document opens a native menu with Open and Copy, and Ctrl+C copies the selection.
- [ ] Later tickets can add a command to the menu in one place.

## Comments

**2026-11-03, review.** In the walking skeleton, the render command (`render_document` in `app/src/document.rs`) reads the one process-wide `CommandLineDocument`. Here a window's document changes with Ctrl+O and dropping, and a window can start without one, so the render command reads the document of the window that calls it, from the per-window state ticket 08 adds. The render command moves into its own module, which finishes splitting `app/src/document.rs` by feature.
