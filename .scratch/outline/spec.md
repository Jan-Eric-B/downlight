# Outline

Status: ready-for-agent

## Problem Statement

In a long document I know the section I want by its name, "Configuration" or "Troubleshooting", but not where it is. Today I scroll and watch for it, or use Find and step through every place the word appears in the text, not just the heading. The document may have a table of contents of its own, but most don't, and when one does it's at the top, far from where I am. I also can't step from one section to the next with the keyboard: Page Down moves by screens, not sections. GitHub, whose reading width Downlight copies, answers this with an Outline that lists the file's headings and filters them as you type. Downlight has nothing like it. The **Overview ruler** will show where the headings are, but finding one by its name still means hovering mark after mark.

## Solution

An **Outline**: a flyout under a new title bar button, as the settings flyout hangs under the gear, that lists the shown document's headings down to h4, indented by level. It opens with the heading at the **Reading position** selected, so it says where I am. Typing filters the list, and Enter or a click goes to the heading, as a heading link does, so Back returns. It opens with Ctrl+Shift+O, its button, or "Outline" in the right-click menu, and closes on a choice, Esc or a click outside. Two keys come with it: Alt+Down and Alt+Up go to the next and previous heading from the reading position, without adding steps to **History**, as F3 adds none. The Outline, the keys and the Overview ruler share one list of headings, h1 to h4. The flyout lies over the document, so nothing reflows and the reading position stays. A docked sidebar was considered and dropped (mockups `mockups.png`, options B and C).

## User Stories

1. As a reader, I want an Outline button in the title bar, so that I can find the list of headings without knowing a shortcut.
2. As a reader, I want the Outline button first among the command buttons, in a group of its own before Open and Edit, so that it sits with the document rather than with the window's switches.
3. As a reader, I want Ctrl+Shift+O to open the Outline, so that I can open it without the mouse.
4. As a reader, I want "Outline" in the right-click menu after Find, with its shortcut beside it, so that the menu teaches the key.
5. As a reader, I want the button's tooltip to say "Outline (Ctrl+Shift+O)", as the other buttons name their keys, so that I learn the shortcut by pointing.
6. As a reader, I want the button shown as pressed while the Outline is open, as the gear is while the settings flyout is, so that I can see what the button opened.
7. As a reader, I want the Outline to hang under its button, right-aligned to it, so that it looks like the settings flyout.
8. As a reader, I want the Outline to lie over the document, so that opening it never rewraps the text or moves my reading position.
9. As a reader, I want the button, Ctrl+Shift+O or the menu entry again to close the Outline, so that the same command opens and closes it.
10. As a reader, I want Esc to close the Outline, so that I can back out without choosing.
11. As a reader, I want a click outside the Outline to close it, so that it gets out of the way when I go back to reading.
12. As a reader, I want opening the settings flyout to close the Outline and opening the Outline to close the settings flyout, so that only one flyout hangs from the title bar at a time.
13. As a reader, I want opening Find to close the Outline, so that typing goes where I expect.
14. As a keyboard reader, I want the focus back where it was when the Outline closes without a choice, on its button if I opened it from there, so that I don't lose my place in the window.

15. As a reader, I want the Outline to list the document's headings from h1 to h4, in the document's order, so that I see its sections and their subsections.
16. As a reader, I want each heading indented by its level, so that I can see which subsections belong to which section.
17. As a reader, I want no h5 and h6 in the list, so that a deeply nested document's list stays short.
18. As a reader, I want headings inside lists, quotes and collapsed sections listed too, so that every section can be reached.
19. As a reader, I want a heading shown as its plain text, inline code and links included as text, so that the list reads like the headings.
20. As a reader, I want a long heading cut with an ellipsis and its full text in a tooltip, so that the flyout stays narrow and I can still read it.
21. As a reader, I want two headings with the same text both listed, each going to its own place, so that a document with a "Usage" under every section works.
22. As a reader, I want a heading written in raw HTML with no anchor left out, as a link couldn't reach it either, so that every row I see leads somewhere.
23. As a reader, I want the Outline about 320 px wide and never taller than the window allows, its list scrolling, so that a long list fits.
24. As a reader, I want the Outline the same size at any zoom, as the title bar is, so that zoom changes the document and nothing else.

25. As a reader, I want the Outline to open with the heading of the section I'm reading selected and scrolled into view, so that it tells me where I am.
26. As a reader at the very start of the document, before its first heading, I want the first heading selected, so that Enter takes me to the first section.
27. As a reader, I want the filter box to have the focus when the Outline opens, so that I can start typing at once.

28. As a reader, I want typing to keep only the headings whose text contains what I typed, so that I find a section by its name.
29. As a reader, I want the filter to ignore case, as Find does, so that "config" finds "Configuration".
30. As a reader, I want the headings that match to keep their indentation and their parents hidden, so that a filtered list stays short.
31. As a reader, I want the first match selected as I type, so that Enter goes to it.
32. As a reader, I want "No headings match" when nothing matches, so that I know the filter, not the document, is empty.
33. As a reader, I want clearing the filter to show the whole list again with the section I'm reading selected, so that I'm back where I started.
34. As a reader, I want the filter box marked with "Filter headings" while empty, so that I know I can type.

35. As a reader, I want Up and Down to move the selection, so that I can choose without the mouse.
36. As a reader, I want Page Up and Page Down to move the selection by a list's height, so that I can cross a long list fast.
37. As a reader, I want the selection to stop at the first and last rows, so that holding a key doesn't wrap me round.
38. As a reader, I want Enter to go to the selected heading and close the Outline, so that one key finishes the job.
39. As a reader, I want a click on a row to go to its heading and close the Outline, so that the mouse works as well as the keys.
40. As a reader, I want the row under the pointer highlighted, so that I can see what a click will choose.
41. As a reader, I want going to a heading to put it at the top of the view, as a heading link does, so that the section starts where I read.
42. As a reader, I want going to a heading in a closed collapsed section to open that section, as a heading link does, so that I land on text I can see.
43. As a reader, I want Back to return me to where I was before I chose a heading, so that a jump through the Outline is as safe as following a heading link.
44. As a reader, I want the focus in the document after I choose a heading, so that the keys scroll the document again.

45. As a reader, I want the button and Ctrl+Shift+O to work in a document without headings, with the Outline saying "No headings in this document", so that I learn why it's empty rather than wondering why a button is grey.
46. As a reader of the empty window, I want the Outline to say "No document", so that it answers there too.
47. As a reader, I want the Outline to say "No document" while a new document is on its way under a **Wait note**, so that it never lists a document I'm not reading.

48. As a reader, I want an open Outline to stay open across a **Reload**, its list updated in place, so that a save in my editor doesn't throw away what I was doing.
49. As a reader, I want a Reload to keep what I typed in the filter and the selected heading if it is still there, so that I can carry on.
50. As a reader, I want the Outline to close when the window goes to another document (a link, Back, Forward, Open, a drop, Paste), so that it never lists a document I'm not reading.

51. As a reader, I want Alt+Down to go to the next heading, h1 to h4, so that I can step through the document section by section.
52. As a reader, I want Alt+Up to go to the start of the section I'm in, and from there to the heading before it, so that it works like a "previous section" key.
53. As a reader, I want the jump keys to go from my reading position, so that they move from what I'm reading, not from where I last clicked.
54. As a reader at the end of a document, where the view can't scroll a late heading to the top, I want Alt+Down to go on to the next heading rather than the same one again, so that I can still step through the last sections.
55. As a reader, I want the jump keys to do nothing past the last or first heading, so that I never wrap round to the other end.
56. As a reader, I want the jump keys to skip headings inside a collapsed section I left closed, so that stepping never opens what I closed.
57. As a reader, I want the jump keys to add no steps to History, as F3 adds none, so that ten steps don't bury the place Back should take me.
58. As a reader, I want holding Alt+Down to keep stepping, as holding F3 does, so that I can run through a document.
59. As a reader, I want the jump keys to work without the Outline open, so that they cost nothing to use.

60. As a reader in **Full screen**, I want the Outline to open with Ctrl+Shift+O at the top of the screen while the title bar is hidden, as the settings flyout does, so that it sits where its button would be.
61. As a reader in full screen, I want the title bar to stay while the Outline I opened is open, as it stays for the settings flyout, so that the flyout doesn't hang from nothing.
62. As a reader in full screen, I want Esc to close the Outline before it leaves full screen, so that Esc closes what's nearest first.

63. As a reader, I want the Outline in both themes and with either **Accent color**, its selection marked in the accent color, so that it fits what I chose.
64. As a reader using a high contrast theme, I want the Outline drawn in the system's colors, so that I can read it and see the selection.
65. As a screen reader user, I want the filter announced as a combo box and its rows as a list box of options, each with its heading's level, so that I can use the Outline without seeing it.
66. As a screen reader user, I want the selected row announced as I move, so that I know where Enter will go.
67. As a reader who prefers reduced motion, I want the Outline to appear at once, so that it doesn't animate.

68. As a reader of a large document, I want the document to appear as soon as it does today, the Outline's list arriving after it, so that the Outline never delays **First paint**.
69. As a reader of a 100 MB document with tens of thousands of headings, I want the Outline to open and to filter as I type without a pause I can feel, so that it is useful at any size.
70. As a reader who opens the Outline before a large document's headings are all listed, I want the headings found so far and a note that more are coming, so that I'm never shown a list that looks complete but isn't.
71. As a reader, I want the Outline to make no network request and read nothing outside the shown document, so that it adds nothing to what Downlight touches.

## Implementation Decisions

- **Page only.** The Outline and the jump keys are page modules. The app and the core don't change, no setting is added, no message or command between the page and the app changes, and no dependency is added. The right-click menu is the page's list that the app shows as given, so its new entry needs no app change.
- **One list of headings, shared.** A page module owns the shown document's list of headings: those from h1 to h4 that have an anchor id, in document order, each with its id, its level, its plain text (the heading's text with runs of whitespace as one space) and the heading element. It is built after first paint, in slices, as Find builds its index, so nothing runs in the frame that puts a document in; it says when it has grown and when it is complete, and is rebuilt when a document is shown, returned to or reloaded. The Overview ruler's ticket 01 creates it (its spec says the list is built "in a form the Outline can reuse") and the Outline only reads it, which is why the Outline is blocked by that ticket. The ruler's marks, the Outline's rows and the jump keys all come from it, so they always agree on what a heading is.
- **The Outline module.** Owns the flyout and its open state. Its interface: open or close it (the `outline` command), whether it is open, and a listener for when it opens or closes, which the title bar uses in full screen as it uses the settings flyout's. It closes the settings flyout when it opens and is closed by the settings flyout and by Find when they open; each side calls the other's public close, neither reaches into the other.
- **Commands.** Three new entries in the page's command table: `outline` ("Outline", Ctrl+Shift+O, a title bar button and a right-click menu entry after Find), `nextHeading` ("Next heading", Alt+Down, repeats while held) and `previousHeading` ("Previous heading", Alt+Up, repeats while held). The jump commands have no button and no menu entry. Ctrl+Shift+O can't reach the webview's own Favorites, since the browser's accelerator keys are off. Alt+Up and Alt+Down are taken by nothing in the page today.
- **The button.** Fluent UI System Icons' "text bullet list" glyph at 16 px, the first command button, in a group of its own before Open and Edit, separated as the other groups are. Its `aria-expanded` is the flyout's open state, set by the Outline module on the button found by its command, as the settings flyout sets the gear's.
- **The flyout's place and size.** It hangs under its button, its right edge level with the button's, about 320 px wide, at most the window's height less a margin, the list scrolling inside it, the filter box staying at its top. Like the title bar, its size doesn't follow the document's zoom. In full screen it follows the settings flyout's rules (full screen spec, stories 20, 47 and 48): at the top of the screen when opened while the bar is hidden, the bar kept shown while it is open, the sliding bar covering it.
- **Opening.** The filter box takes the focus; the selected row is the heading of the section that holds the reading position: the last heading at or before the block at the top of the view, compared by place in the document, as Find's first current match is found (not by line number, which footnotes and raw HTML break). Before the first heading, the first row is selected. The selected row is scrolled into the middle of the list.
- **Rows.** One row per heading: its plain text, indented by level (h1 none, each level a step further), cut with an ellipsis, its full text in the row's tooltip. The selected row is marked with the accent color's selection, the row under the pointer with the subtle hover fill the title bar's buttons use.
- **Only the rows near the list's view exist.** The list has a fixed row height and holds row elements only for the rows in its view and a margin around them, sized as the whole list so its scroll bar is true. Opening the Outline and each keystroke in the filter then cost the rows shown plus one pass over the headings' text, however many headings the document has.
- **Filter.** A row is kept when its heading's text, folded as Find folds text (case, final sigma), contains the filter's text folded the same way. Kept rows keep their indentation; their parents are hidden. After each change the first kept row is selected; an empty filter brings back the whole list with the reading position's heading selected. None kept shows "No headings match" in the list.
- **Keys in the flyout.** Up and Down move the selection by a row and Page Up and Page Down by the list's height, stopping at the ends; the list scrolls to keep the selection in view. Enter chooses. Esc closes. Every other key goes to the filter box, and the keys that edit text there (typing, Ctrl+A, Ctrl+C, Ctrl+V, Ctrl+Z) act in the box, as they do in Find's.
- **Choosing.** Enter or a click on a row closes the Outline, puts the focus in the document and goes to the heading through the view's `visitHeading`, as a heading link and a heading mark of the ruler do: it opens the collapsed sections around it, scrolls it to the top and lands on it as ADR 0003 asks, and joins History.
- **Closing without a choice.** Esc, the command again, a pointer down outside the flyout, opening the settings flyout or opening Find close it. Esc and the command return the focus to the button when the Outline was opened from it, and to the document otherwise, as the settings flyout does with the gear.
- **Esc's order.** The open Outline takes Esc first, in the settings flyout's place: the two are never open together. If image lightbox ticket 01 (one place owns Esc's order) is done, the Outline registers there; otherwise it defers as the settings flyout's listener does today and the later listeners (Find, Stop, full screen) learn to defer to it.
- **Nothing to list.** In a document whose list is complete and empty, the flyout shows "No headings in this document" and no filter results. In the empty window, and while a Wait note for a new document shows, it shows "No document". In both the filter box is still there but disabled.
- **A list still being built.** On a large document, the flyout shows the headings listed so far and a last row "Listing headings..." until the list is complete; it grows in place, keeping the selection and the filter. Jump keys use what is listed so far.
- **Reload and other documents.** On a reload (`whenDocumentShown` with reason `reload`) an open Outline stays open, rebuilds its rows from the new list, keeps the filter text, and keeps the selected heading by its id, or else selects the first kept row. On a new document or a return (reasons `shown` and `return`), and when a Wait note for a new document appears, it closes.
- **Jump keys.** Next heading goes to the first listed heading after the reading position's block, not inside it; previous goes to the last listed heading before it, so from inside a section it first returns to that section's heading. A heading inside a closed collapsed section (`details` without `open`) is skipped. Past the ends, nothing happens. Because the view can't scroll a heading near the document's end to the top, the reading position may not reach the heading a jump went to: while the view hasn't scrolled since the last jump, the next jump steps from the heading that jump went to rather than from the reading position. The view gains a way to scroll to a heading as a link does without joining History (the jump keys' own entry point beside `visitHeading`).
- **Accessibility.** The filter box is a combo box (`role="combobox"`, `aria-expanded`, `aria-controls` naming the list, `aria-activedescendant` naming the selected row); the list is a list box of options, each option's accessible name its heading's text with its level ("Configuration, level 2"). The empty and "Listing headings..." notes are a live status. Under forced colors the flyout uses the system's colors, the selected row the highlight colors, as Find's highlights do.
- **Glossary.** **Outline** was added in the grilling. The jump keys need no term of their own: their commands are "Next heading" and "Previous heading".

## Testing Decisions

- **A good test** drives the page as a reader does and checks what a reader can observe: which rows the Outline shows, their text and level, which row is selected, what the empty notes say, where the view scrolled, which heading is at the top, whether Back returns, where the focus is, and whether the button shows pressed. It never reads the Outline's internal state or the list of headings module directly. Names state the behavior, such as `choosing a heading goes there and Back returns` or `Alt+Down at the end of the document goes on to the next heading`.
- **One seam: the page** (Vitest with happy-dom), through the existing fakes: `fakeTheApp` to show a document, reload it (`saveTheFile`), give another one and step through History; keyboard events for Ctrl+Shift+O, Alt+Up and Alt+Down, typing in the filter, Up, Down, Page Up, Page Down, Enter and Esc; clicks on the button, on rows and outside; the right-click menu's entries as the app is asked to show them; `layOut` for the geometry the reading position and the scroll land on; the full screen helpers for the bar.
- **A new rendered fixture**, made by the core as the others are and reviewed in its diff: headings h1 to h6, two headings with the same text, a heading inside a list, a quote and a closed `details`, a raw HTML heading without an id, a heading with inline code and a link, and a long heading. The Overview ruler's tests use it too.
- **Cases** include: the button, the shortcut and the menu entry each open it, and the command again closes it; the rows and their indentation, none for h5 and h6 or the raw HTML heading; the reading position's heading selected on opening, the first heading before any; filtering, ignoring case, parents hidden, the first match selected, "No headings match", clearing; Up, Down and the page keys, stopping at the ends; Enter and a click going there and closing, Back returning; a heading in a closed section opened; Esc and a click outside closing without moving the view, the focus back on the button; the settings flyout and Find closing it and it closing the settings flyout; "No headings in this document", "No document" in the empty window and during a Wait note; a reload keeping it open with its filter and selection, a new document and Back closing it; the jump keys forward and back, from inside a section, skipping a closed section, doing nothing past the ends, going on at the document's end, adding no History steps; full screen keeping the bar shown while it is open and Esc closing it before leaving full screen.
- **No app or core tests**: neither changes. The contract samples don't change.
- **Prior art:** the settings flyout's page tests (a flyout under a title bar button, its focus and Esc, a pointer down outside), Find's page tests (typing, folding, the current match at the reading position against `layOut`), the History tests (Back after a heading link), `title-bar-full-screen.test.ts` (the bar kept by a flyout), `commands.test.ts` (shortcuts, menu entries, tooltips naming keys) and `wait-note.test.ts` (Esc's order with Stop).
- **Measurement, not a test:** with the timing switch on the 1 MB test document and then the 20 and 100 MB documents (`npm run measure -- large`): the frame that puts the document in is no longer than today (ticket 02's baseline); opening the Outline, typing a letter in its filter and holding Alt+Down keep every page task and frame under 100 ms; note when the list of headings is complete. The results go in the ticket.

## Out of Scope

- **A docked sidebar** or pinning the Outline beside the document (mockup options B and C), and with it a heading mark that follows scrolling: dropped in the grilling, along with the v1 spec's Later item "a table of contents sidebar".
- **The current heading in the title bar** (breadcrumbs) and **a sticky heading at the top of the view** (mockup option D).
- **h5 and h6**, and headings in raw HTML without an anchor id.
- **Fuzzy matching** or matching the document's text rather than its headings: Find does that.
- **Folding sections** of the document from the Outline, or numbering headings.
- **Menu entries or buttons for the jump keys**, and jump keys for one level only (next h2).
- **Remembering the filter** or the Outline's open state across documents, windows or restarts.
- **Changes to the app, the core, the settings or the contract.**

## Further Notes

- Decided in the grilling of 2026-11-20 (the questions and answers, with the options in `mockups.png`). The same grilling moved the Overview ruler's heading marks from h1 to h3 to h1 to h4 (its spec and ticket 01 were updated) and removed the sidebar from the v1 spec's Later list.
- The jump keys need only the list of headings and the view's new scroll without a History step, so they are the natural first ticket, ahead of the flyout.
- The flyout's opening, filtering and rows are the parts that grow with a document's size, so they are measured on the large documents when built (coding standards, Responsiveness).
- Esc's order is being given one owner by image lightbox ticket 01; whichever of the two lands second adapts to the first.
