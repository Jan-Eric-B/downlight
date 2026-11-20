# 01: Alt+Down and Alt+Up step through the headings

**What to build:** Alt+Down goes to the next heading of the shown document and Alt+Up to the previous one, h1 to h4, from the **Reading position**, as a heading link scrolls to a heading but without adding a step to **History**, as F3 adds none. They need no Outline open: they are the first slice of the **Outline**'s spec.

Spec: [Outline](../spec.md)

**Blocked by:** [ruler/01](../../overview-ruler/issues/01-heading-marks-beside-the-scroll-bar.md) (Heading marks beside the scroll bar), which builds the shared list of headings, h1 to h4 with an anchor id

**Status:** ready-for-agent

- [ ] Two commands in the page's command table, "Next heading" (Alt+Down) and "Previous heading" (Alt+Up), both repeating while held as Find next does, with no title bar button and no right-click menu entry.
- [ ] Next heading goes to the first listed heading after the block at the reading position, not inside it; Previous heading goes to the last listed heading before it, so from inside a section it first returns to that section's heading. Both compare places in the document, not line numbers, as Find's first current match does.
- [ ] Both read the shared list of headings only (h1 to h4 with an anchor id); h5, h6 and raw HTML headings without an id are never stepped to.
- [ ] A heading inside a closed collapsed section (`details` without `open`) is skipped, and no section is opened.
- [ ] Past the last or the first heading nothing happens: the view and History stay as they are, and nothing wraps round.
- [ ] At the document's end, where the view can't scroll a heading to the top and the reading position stays short of it: while the view hasn't scrolled since the last jump, the next jump steps from the heading that jump went to, so repeated Alt+Down goes on through the last headings.
- [ ] The view gains a way to scroll to a heading as a link does (the heading at the top, its landing checked as ADR 0003 asks) without joining History, beside `visitHeading`.
- [ ] A new rendered fixture document, unless ruler/01 already added it: headings h1 to h6, two headings with the same text, a heading inside a list, a quote and a closed `details`, a raw HTML heading without an id, a heading with inline code and a link, and a long heading. Written with `npm run fixtures`, its diff reviewed.
- [ ] Page tests (written first): Alt+Down and Alt+Up go to the next and previous heading; from inside a section Alt+Up returns to its heading first; h5, h6 and the raw HTML heading are skipped; a heading in a closed section is skipped and the section stays closed; past the ends nothing changes; at the document's end Alt+Down goes on to the next heading; Back after several jumps returns to where the last heading link left from, not to a jump.
- [ ] `npm run check` passes.
