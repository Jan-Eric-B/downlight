# 05: The Outline in full screen, and its place in Esc's order

**What to build:** In **Full screen** the **Outline** behaves as the settings flyout does there: opened with Ctrl+Shift+O while the title bar is hidden, it sits at the top of the screen; while it is open the bar stays shown; and Esc closes it before Find, Stop or full screen take the key.

Spec: [Outline](../spec.md)

**Blocked by:** 02 (The Outline lists the headings and goes to one)

**Status:** ready-for-agent

- [ ] The Outline module tells listeners when it opens and closes; the title bar keeps itself shown in full screen while the Outline is open, as it does for the settings flyout (full screen spec, stories 20 and 21).
- [ ] Opened while the bar is hidden, the flyout starts at the top of the screen; the sliding bar covers it while it shows rather than pushing it down (full screen spec, stories 47 and 48).
- [ ] Esc's order: the open Outline takes Esc in the settings flyout's place, before Find, Stop and full screen. If image-lightbox/01 (one place owns Esc's order) is done, the Outline registers in that one list; otherwise Find's, Stop's and full screen's Esc listeners defer to the open Outline as they defer to the settings flyout today. Whichever of the two tickets lands second adapts to the first.
- [ ] Page tests (written first): in full screen, Ctrl+Shift+O opens it at the top of the screen, and the bar shows while it is open and hides once it closes and nothing else keeps it; Esc with the Outline open closes it and leaves full screen, Find and a stoppable Wait note as they are; the next Esc goes on down the order.
- [ ] `npm run check` passes.
