# 38: A large document's load can be left

**What to build:** While a large document loads, the reader can still close, move and leave the window: the wait note says how long it has been going and has a button that stops the load, and a link, Back or Open goes elsewhere instead of waiting.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** needs-triage

- [ ] Grill it before planning. Open questions below.
- [ ] Measured with the timing switch on `mixed-20mb.md` and `mixed-100mb.md` (`npm run documents -- large`), against ticket 31's Results.

## Comments

**2026-11-19, from the repo owner's hand checks of ticket 31.** Opening a large document (20 MB, 100 MB) freezes the whole window until it shows: the reader can't close it, scroll, follow another link or cancel. The owner asked for a time display in the wait note, perhaps a percentage of what is loaded or the memory it takes, and an "x" that stops the load.

**What freezes, from ticket 31's Results** (100 MB mixed, loaded machine): read 42 ms, render 13.3 s, IPC 2.5 s, insert 5.5 s, frame after the insert 9.6 s, longest page task or frame 11.7 s, memory 3.8 GB.

- The render runs on the app's blocking thread, so the page is free during it (13 s at 100 MB). What the reader can do then is untested: the wait note sits over the old document, and whether a link, Back or the close button work then is a hand check.
- The insert and the frame after it run on the page's main thread (about 15 s at 100 MB, about 3 s at 20 MB), and ADR 0003's `content-visibility: auto` already skips layout of blocks away from the view. While that thread is busy the page answers nothing, and the title bar is the page's (ADR 0005), so its close button, its drag region and every key are dead too. Closing from the taskbar (right-click, Close) or Alt+F4 goes through the app's window, not the page, and may still work; check it before planning.

**Open questions, with recommendations:**

1. **A percentage or the memory taken: recommended against.** Neither stage reports progress: comrak renders the whole document in one call, and the page inserts the HTML in one operation, so a percentage would be made up, and a figure in MB says nothing a reader acts on. Elapsed seconds in the note ("Showing **a.md** (100 MB) ... 12 s") are honest and cheap.
2. **A Stop button on the wait note.** It shows the previous document again, or the empty window. The render can't be interrupted inside comrak, but the app can drop its result. Whether the page can drop an insert it has started depends on question 3.
3. **A page that answers during the insert.** This is the hard part and the main thing the owner hit. The way found so far is ADR 0003's rejected option: insert the document in chunks over several frames. Find, ticks, reload and the reading position would then wait for the last chunk, so it supersedes part of ADR 0003. A cheaper half-step: insert in chunks only above a size (say 5 MB), where a frozen window is worse than a document that shows in stages.
4. **In v1 or after?** It affects only documents of several MB. A 1 MB document never freezes the window for more than 100 ms (ticket 02's target). Recommendation: after v1, unless closing from the taskbar fails too. A window that can't be closed at all is worse than one that is slow.
