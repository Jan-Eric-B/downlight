# 01: One place owns Esc's order

**What to build:** A prefactor with no change the reader sees. Today Esc's order (the settings flyout, Find, Stop, then **Full screen**) is spread over each feature's own keydown listener, each asking the others whether they are open before it acts. Give the order one owner: a single Esc listener that walks an ordered list of the features that take Esc, asking each whether it takes it now and giving the key to the first that does. Each feature registers itself with its place in the order instead of knowing the others. The **Lightbox** (ticket 02) then joins the list in one place, after the settings flyout and before Find.

Spec: [Image lightbox](../spec.md) (Further Notes, and Implementation Decisions, "Esc")

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] One listener owns Esc: it ignores a composing event, one with Ctrl, Alt, Shift or the Windows key held, and one already taken, and otherwise gives the key to the first feature in the order that takes it now, preventing its default.
- [ ] The order is written down once, in that one place: the settings flyout, Find, Stop, full screen, with the place for the lightbox named in a comment.
- [ ] The settings flyout, Find, Stop and full screen each take Esc only through it; none of their listeners asks another feature whether it is open any more.
- [ ] A held Esc's repeat still never switches full screen back (it switches on the app's answer), and Stop still goes on only while the Wait note can stop.
- [ ] The existing tests of Esc's order (Find, the Wait note, full screen, the settings flyout) pass unchanged; if one has to change, say which and why.
- [ ] `npm run check` passes.
