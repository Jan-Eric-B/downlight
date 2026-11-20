# 01: The macOS port

**What to build:** Downlight runs on macOS as it does on Windows, with macOS's vibrancy behind the document where Windows has Mica.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later ("the macOS port with vibrancy").

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - What the per-OS module must give on macOS: the **Editor** and **Text file program** (Launch Services), revealing a file in Finder, **Backdrop** as vibrancy, **Keep on top**, the **Accent color** from the system.
  - The **Title bar**: the page draws it on Windows (ADR 0005); on macOS the traffic lights and their place, full screen, and the menu bar every Mac app has.
  - File associations, "Open With", and opening a file when Downlight is already running (Apple events, not a command line).
  - Shortcuts in Cmd, not Ctrl, and **Network share** rules for SMB mounts.
  - Signing, notarization, a DMG or the App Store, and Apple Silicon and Intel builds; CI on macOS.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). The v1 spec's story 84 keeps per-OS code in one module per OS so this port touches only those. Not yet grilled.
