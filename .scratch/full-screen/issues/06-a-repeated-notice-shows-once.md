# 06: A repeated Notice shows once

**What to build:** A bug fix. Pressing F11 quickly several times stacks the full screen **Notice** "Press F11 or Esc to leave full screen" in the window's notice at the bottom, the same line two or three times, one above the other: every answer that the window is now full screen adds the line again, and the notice keeps up to three lines. Different messages stacking is right (several problems at once each get their line), but a message the notice already shows must not show twice. Instead, the line already there stays and the notice's 4 s start again, as if it had just been shown.

Spec: [Full screen](../spec.md) (story 40, Implementation Decisions, "The Notice"), and the window design spec's notices

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] A failing page test first, reproducing the bug: switching full screen on, off and on again within the notice's 4 s (the fake app answering each switch) leaves exactly one "Press F11 or Esc to leave full screen" line in the notice.
- [ ] Showing a Notice whose text the notice already shows adds no line; the existing line moves to the end, as the newest, and the notice stays 4 s from now. This holds for every Notice, not only full screen's, so a reader who repeats any failing action (a drop of the same file, Edit with no editor) sees its message once.
- [ ] Two Notices with different text still stack, up to three lines, the oldest dropped first, as today.
- [ ] A Notice with a file in it counts as the same only when its text and its file are the same; the same message about two different files is two lines.
- [ ] A notice held by the pointer or the focus on its folder link is held as before.
- [ ] Page tests: the reproduction above; the same plain message shown twice gives one line and restarts the 4 s; two different messages give two lines; the same message about two files gives two lines.
- [ ] `npm run check` passes.
