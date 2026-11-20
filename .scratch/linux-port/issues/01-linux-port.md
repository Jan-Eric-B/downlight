# 01: The Linux port

**What to build:** Downlight runs on Linux desktops as it does on Windows, with a solid background where Windows has Mica.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later.

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - The webview is WebKitGTK, a package dependency, not Chromium: check every page feature the Windows build relies on (CSS Custom Highlights for Find, `content-visibility` for ADR 0003, `light-dark()`), and its speed on the 1 MB document.
  - What the per-OS module must give: the **Editor** and **Text file program** (XDG MIME defaults), revealing a file, **Keep on top**, the **Accent color** and **Theme** from the desktop.
  - The **Title bar**: page-drawn (ADR 0005) or the desktop's own, under GNOME, KDE and Wayland.
  - Packaging: AppImage, deb, rpm, Flatpak; file associations through desktop entries; CI on Linux.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). The plain fallbacks in the app's platform module are what Linux gets until this port. Not yet grilled.
