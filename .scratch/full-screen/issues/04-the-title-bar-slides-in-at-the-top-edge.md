# 04: The title bar slides in at the top edge

**What to build:** In **Full screen** the hidden **Title bar** slides in over the document while the pointer rests at the top edge of the screen, and slides away once the reader is done with it. It lies over the document, so the reading position never moves, and covers the Find bar and the settings flyout while it shows. Its logo, file name (with its path and **Counts** tooltip, and reveal on a click), zoom level and buttons work as outside full screen.

Spec: [Full screen](../spec.md)

**Blocked by:** 03 (The title bar hides in full screen)

**Status:** ready-for-agent

- [ ] In full screen, the bar shows while any of these holds: the pointer is in a thin zone at the top edge of the screen or over the bar, the focus is within the bar, or the settings flyout is open. It hides once none holds. Its tooltips are the system's and show only with the pointer over the bar, which that rule covers.
- [ ] Shown, the bar keeps its own height and lies over the document; the room kept for it stays zero, so nothing under it moves.
- [ ] It slides in and out, and appears and disappears at once when the system asks for reduced motion.
- [ ] Out of full screen the bar is as today, whatever the pointer or the focus does.
- [ ] Page tests (written first): in full screen the pointer entering the top zone shows the bar and leaving the zone and the bar hides it; the focus moving into the bar shows it and out hides it; the open settings flyout keeps it, and closing the flyout with the pointer and focus elsewhere hides it; the room kept for the bar stays zero while it shows.
- [ ] Hand check in the release build: the bar slides in at the top edge and away again; moving from the gear into the flyout keeps it; Tab into the bar shows it; the file name's tooltip shows the path and Counts; with reduced motion it appears at once; the document doesn't move.
- [ ] `npm run check` passes.
