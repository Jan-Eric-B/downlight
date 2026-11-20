# 01: Mermaid diagrams and math

**What to build:** Mermaid code blocks show as diagrams and `$...$` and `$$...$$` math shows typeset, as GitHub shows them, with the code that does it loaded only for documents that have them.

Spec: none yet. Grill it first (`/grill-with-docs`). From the v1 spec's Out of Scope, Later.

**Blocked by:** None (can start immediately)

**Status:** needs-triage

- [ ] Grill it before planning. Open questions:
  - Which libraries (Mermaid, KaTeX or MathJax), their size and licenses (GPL-3.0 compatible), bundled so no network request is made.
  - Who renders: the core marks the blocks (comrak has a math extension), the page draws them. Diagrams come from an untrusted document: Mermaid's strict security level, and the SVG sanitized again before it reaches the page.
  - A diagram that fails: its code block stays, with or without a **Notice**?
  - Large documents: a cap on diagrams or their time, like the **Highlight limit**, and drawing only diagrams near the view (ADR 0003).
  - Theme: the diagrams' colors in dark and light, and a **Reload** keeping the **Reading position** when diagrams change height.

## Comments

**2026-11-20.** Stub from the v1 spec's Out of Scope list (Later). mdhero renders Mermaid (strict, sanitized again) and KaTeX (`.scratch/mdhero-comparison/features.md`). Not yet grilled.
