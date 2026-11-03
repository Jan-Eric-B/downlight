# 14: Find

**What to build:** Ctrl+F opens a find bar that highlights every match in the shown document as the reader types, shows "3 of 12", and steps through the matches. **Find** looks only in the shown document.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 03 (Page test harness), 09 (Opening from inside a window)

**Status:** ready-for-agent

- [ ] Ctrl+F opens the find bar with the focus in its box; pressing it again while open selects the text in the box.
- [ ] Matches are highlighted as the reader types, ignoring case, without changing the document's DOM.
- [ ] The bar shows "N of M", or "No results" when nothing matches.
- [ ] Enter and F3 go to the next match, Shift+Enter and Shift+F3 to the previous one, wrapping at the ends. The current match scrolls into view and is highlighted differently from the rest.
- [ ] Esc closes the bar and clears the highlights.
- [ ] After a reload while the bar is open, the matches are counted again.
- [ ] Find is in the right-click menu.
- [ ] Page tests cover the count, the current match, next and previous, wrapping, Esc and the count after a reload.
- [ ] Typing in the box on the 1 MB document is measured with the timing switch, and the result is recorded in this ticket.
