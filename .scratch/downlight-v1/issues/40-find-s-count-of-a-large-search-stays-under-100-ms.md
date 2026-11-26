# 40: Find's count of a large search stays under 100 ms

**What to build:** Typing a one-letter search in Find on the 1 MB mixed test document keeps every page task and frame under 100 ms. Today the first count of "e" (about 105,000 matches) is one task of 49 to 150 ms, then 25 to 43 ms of style, layout and paint for its highlights, with or without the Overview ruler.

Spec: [Downlight v1](../spec.md)

**Blocked by:** none

**Status:** needs-triage

Found while measuring the Overview ruler (ruler ticket 01's Results, "2026-11-26, diagnosed"):

- Most of the task is building one `StaticRange` per match for the highlight: 32 to 125 ms for "e".
- Garbage collection decides whether it crosses 100 ms: while the garbage the document's arrival left is still held, the highlight for "e" takes 47 to 67 ms; once collected, 36 to 40 ms.

Open question for the grilling: build the highlight of a large count in slices over several tasks, or highlight only the matches near the view (and what the Overview ruler's Find marks, ruler ticket 03, then read). Measured with `npm run measure` against the numbers above.
