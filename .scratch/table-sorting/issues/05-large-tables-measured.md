# 05: Large tables, measured

**What to build:** Sorting a large table (about 10,000 rows) keeps the window answering: every press, and the step back to the file's order, keeps every task and frame under 100 ms, with Find open or closed, and the sort buttons cost the first paint of a document with many tables nothing the reader can feel.

Spec: [Sort a document's tables](../spec.md)

**Blocked by:** 01 (Sort a table by a column, in natural order), 02 (Number columns sort by value), 04 (Find, the ruler, copying and Edit follow the shown order)

**Status:** ready-for-agent

- [ ] `npm run documents` also writes a test document of about 1 MB that is one table of about 10,000 rows, with a number column and a text column, from the generator's fixed seed. The existing test documents stay as they are, so ticket 02's baseline (downlight-v1) and ticket 31's results still compare.
- [ ] `npm run measure` opens it with the timing switch and presses each sort (ascending, descending, the file's order; on the number column and on the text column), once with the find bar closed and once open on a common word, and reports the longest task or frame for each, as medians of 5 runs.
- [ ] Every press keeps every task and frame under 100 ms. If one can't, stop and revisit the spec (slices, a cheaper key read or Find's index refresh) before shipping a slower sort.
- [ ] First paint and the frame after the document goes in on the 1 MB mixed document (a table about every 30 lines) stay within the spread of ticket 02's baseline.
- [ ] The results go in this ticket under `## Results`, with the machine and the build.
- [ ] Doesn't wait for v1 ticket 39.
- [ ] `npm run check` passes.
