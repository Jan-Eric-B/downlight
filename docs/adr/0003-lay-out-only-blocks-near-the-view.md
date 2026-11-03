# Lay out only the blocks near the view

The page gives each top-level block of the rendered document `content-visibility: auto` with `contain-intrinsic-size: auto 3em`, so the webview lays out and paints only the blocks near the view and skips the rest until they scroll close. We chose this because the spec's 1 MB target is that the window never stops answering for more than 100 ms. The performance spike (ticket 02) measured the skeleton without it: the frame after inserting a 1 MB document did style and layout for the whole document and took 141 to 186 ms. With it, that frame takes 25 to 50 ms, first paint of a 1 MB document is 70 to 100 ms sooner, and scrolling through the whole document gave no task or frame over 100 ms.

## Considered options

- **Lay out the whole document at once**, as the skeleton did. Every block's position is exact from the first frame, but a 1 MB document holds up the page for well over 100 ms.
- **Insert the document in chunks over several frames.** Keeps each frame short and the heights exact once all chunks are in, but needs page code to schedule the chunks, shows the document in stages, and makes Find, ticks and reload wait for the last chunk.

## Consequences

- Until a block has been shown, its height is an estimate, and the browser remembers the real height once it has laid the block out. The document's height, and so the scroll bar, can shift a little while reading; on the 1 MB test documents the estimate was within 1 % of the real height.
- Code that scrolls to a block the reader hasn't seen yet (the reading position after a reload, links to headings, Find's current match, Back and Forward) can't rely on its offset before scrolling to it, and has to check that it lands where it should.
- The overview ruler can't place its marks from block offsets until every block has been laid out once.
- The rule sits in `page/src/document.css`. Changing the document's top-level structure (wrapping blocks in sections, for example) moves where it has to apply.
