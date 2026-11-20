# 01: The Explorer preview pane

**What to build:** Selecting a Markdown file in File Explorer shows it rendered in Explorer's preview pane, as Downlight shows it.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later.

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - A preview handler is a COM component that Explorer loads in its own host process, not a Tauri window. Does it host WebView2 with the core's rendering, or show something simpler? It probably needs its own ADR.
  - What it shows: the rendered document only, with links, **Local images** and Find off? Its theme and **Backdrop**?
  - Safety: it renders untrusted documents whenever a file is merely selected, so the core's sanitizing and **Network share** rules hold there too, and remote images likely stay off.
  - The installer registers it per user, and uninstalling removes it.
  - Large files: a size above which it shows nothing, since Explorer waits on it.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). Not yet grilled.
