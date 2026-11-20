# 01: Printing and PDF

**What to build:** The reader can print the shown document, or save it as a PDF, through the webview's print dialog, laid out for paper.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, v1.x ("printing and PDF through the webview's print dialog").

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Print styles: always light on white whatever the **Theme**, no **Backdrop**, no title bar, find bar, **Overview ruler** or copy buttons; wrapped code; no page breaks inside tables, code and images. Which of GitHub's styles carry over?
  - PDF: Windows' "Microsoft Print to PDF" in the same dialog, or a direct "Save as PDF" through WebView2's own PDF printing?
  - ADR 0003's skipped blocks: check that blocks never laid out print at their real size, and how long printing a 1 MB and a 100 MB document takes.
  - Collapsed sections, **Ticks** and a **Sort**: printed as shown?
  - How it starts: Ctrl+P, a right-click entry.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (v1.x). mdhero has print styles and PDF (`.scratch/mdhero-comparison/features.md`). Not yet grilled.
