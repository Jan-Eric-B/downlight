# 01: Export to HTML

**What to build:** The reader can save the shown document as one standalone HTML file that looks as Downlight shows it, to send to someone or publish, with no Downlight needed to open it.

Spec: none yet. Grill it first (`/grill-with-docs`).

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Standalone how: styles inlined (GitHub's stylesheet and the highlighting), **Local images** embedded as data or copied beside it, remote images left as links?
  - Light, dark or both (following the viewer's system with `prefers-color-scheme`)? Downlight's **Accent color** or GitHub's?
  - What is left out: the title bar, Find, copy buttons, the **Overview ruler**; **Ticks** and a **Sort** as shown or as in the file?
  - It writes a file, a first for Downlight ("reading only"): a save dialog, and the glossary's opening line may need a word on it.
  - How it starts (a right-click entry, Ctrl+Shift+S), and how it relates to printing and PDF (`.scratch/print/issues/01-print-and-pdf.md`), which shares the "laid out for others" styles.

## Comments

**2026-11-20, from the repo owner.** Picked from the mdhero comparison (`.scratch/mdhero-comparison/features.md`, "Export to HTML", planned in mdhero).
