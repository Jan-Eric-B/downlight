# 02: The Outline lists the headings and goes to one

**What to build:** An Outline button in the title bar, Ctrl+Shift+O and an "Outline" entry in the right-click menu open the **Outline**: a flyout under the button, as the settings flyout hangs under the gear, listing the shown document's headings from h1 to h4, indented by level, with the heading of the section at the **Reading position** selected. The reader moves the selection with the keys and chooses a heading with Enter or a click, which goes there as a heading link does, so Back returns. Filtering comes in ticket 03.

Spec: [Outline](../spec.md)

**Blocked by:** [ruler/01](../../overview-ruler/issues/01-heading-marks-beside-the-scroll-bar.md) (Heading marks beside the scroll bar), which builds the shared list of headings

**Status:** ready-for-agent

- [ ] An `outline` command, "Outline", Ctrl+Shift+O: its button, Fluent UI System Icons' "text bullet list" at 16 px, is the first command button, in a group of its own before Open and Edit; its menu entry comes after Find. The tooltip reads "Outline (Ctrl+Shift+O)". The command opens the Outline, or closes it when open.
- [ ] The button's `aria-expanded` follows the flyout's open state, set by the Outline module on the button found by its command, and the button shows pressed while it is open, as the gear does.
- [ ] The flyout hangs under its button, its right edge level with the button's, about 320 px wide, at most the window's height less a margin, its list scrolling, its size not following the document's zoom. It lies over the document: opening and closing it never moves the reading position.
- [ ] Rows come from the shared list of headings: one per heading from h1 to h4 with an anchor id, in document order, its plain text (inline code and links as text), indented by level, cut with an ellipsis, its full text in the row's tooltip. Two headings with the same text are two rows, each going to its own heading.
- [ ] Only the rows in the list's view and a margin around them exist as elements; the list is sized as the whole list, with a fixed row height, so its scroll bar is true.
- [ ] On opening, the selected row is the heading of the section holding the reading position (the last heading at or before the block at the top of the view, by place in the document), or the first row before any heading, scrolled into the middle of the list. The filter box (marked "Filter headings", disabled until ticket 03) has the focus.
- [ ] Up and Down move the selection by a row, Page Up and Page Down by the list's height, stopping at the ends; the list keeps the selection in view. The row under the pointer is highlighted with the title bar buttons' hover fill, the selected row with the accent color's selection.
- [ ] Enter, or a click on a row, closes the Outline, puts the focus in the document and goes to the heading through the view's `visitHeading`: it opens the collapsed sections around it, puts it at the top of the view and joins **History**.
- [ ] Esc, the command again, a pointer down outside the flyout, opening the settings flyout and opening Find close it without moving the view. Opening the Outline closes the settings flyout. Each side calls the other's public close. Esc and the command return the focus to the button when the Outline was opened from it, and to the document otherwise.
- [ ] Esc here takes the settings flyout's place in Esc's order (the two are never open together); full screen's own rules are ticket 05.
- [ ] The filter box is a combo box (`aria-expanded`, `aria-controls` naming the list, `aria-activedescendant` naming the selected row); the list is a list box of options, each named by its heading's text and level ("Configuration, level 2").
- [ ] It looks right in both themes and with either **Accent color**; under forced colors it uses the system's colors, the selected row the highlight colors. It appears at once, with no animation.
- [ ] Page tests (written first): the button, Ctrl+Shift+O and the menu entry open it and the command closes it; the rows, their levels and their order, none for h5, h6 or a raw HTML heading; the reading position's heading selected, the first row before any heading; Up, Down, Page Up and Page Down stopping at the ends; Enter and a click going there, closing, and Back returning; a heading in a closed section opened; Esc and a click outside closing without moving the view, the focus back on the button; the settings flyout and Find closing it, and it closing the settings flyout; the button's pressed state and tooltip; only rows near the list's view exist in a long list.
- [ ] `npm run check` passes.
