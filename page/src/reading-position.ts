/**
 * The reading position, and every scroll that has to land where it aims. The page gives the
 * document's blocks `content-visibility: auto` (ADR 0003), so a block the reader hasn't seen yet
 * has an estimated height until a frame lays it out, and a scroll to it can land a little off.
 * Reload, heading links and Find all scroll here, and check after a frame that they landed.
 */

/**
 * Resolves once the next frame is done: the task queued from `requestAnimationFrame` runs after
 * that frame's style, layout and paint.
 */
export function afterNextFrame(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      setTimeout(resolve, 0);
    });
  });
}

/**
 * Scrolls with `scroll`, and once the next frame has laid out the blocks it brought near the
 * view, corrects with `correct` if `stillWanted` says the scroll is still wanted then. Returns
 * whether it was.
 */
async function scrollAndLand(
  scroll: () => void,
  correct: () => void,
  stillWanted: () => boolean,
): Promise<boolean> {
  scroll();
  await afterNextFrame();
  if (!stillWanted()) return false;
  correct();
  return true;
}

/**
 * Scrolls `element` to the top of the view, and once more after the next frame if `stillWanted`
 * says the scroll is still wanted then. Returns whether it was.
 */
export function scrollToTop(
  element: Element,
  stillWanted: () => boolean,
): Promise<boolean> {
  const scroll = () => {
    element.scrollIntoView({ block: "start" });
  };
  return scrollAndLand(scroll, scroll, stillWanted);
}

/**
 * Scrolls the block holding `range` to the middle of the view. After the next frame, if
 * `stillWanted` says the scroll is still wanted then, it scrolls `range` itself to the middle if
 * it is outside the view: the block can be a paragraph taller than the view, so it's the range
 * that the reader has to see.
 */
export async function scrollToMiddle(
  range: StaticRangeInit,
  stillWanted: () => boolean,
): Promise<void> {
  const holder = range.startContainer.parentElement;
  if (!holder) return;
  await scrollAndLand(
    () => {
      holder.scrollIntoView({ block: "center" });
    },
    () => {
      const live = document.createRange();
      live.setStart(range.startContainer, range.startOffset);
      live.setEnd(range.endContainer, range.endOffset);
      const { top, bottom } = live.getBoundingClientRect();
      if (top < 0 || bottom > innerHeight) {
        scrollBy(0, (top + bottom - innerHeight) / 2);
      }
    },
    () => stillWanted() && holder.isConnected,
  );
}

/** The line of the file where `block` starts, from the core's `data-sourcepos`. */
function startLine(block: Element): number | undefined {
  const position = block.getAttribute("data-sourcepos");
  if (position === null) return undefined;
  const line = Number.parseInt(position);
  return Number.isNaN(line) ? undefined : line;
}

/**
 * The first of `elements` that isn't wholly above the top of the view. They are laid out top to
 * bottom, so a binary search finds it with few layout reads, even among the 1 MB document's
 * thousands of blocks.
 */
function firstNotAboveTop(elements: HTMLCollection): Element | undefined {
  let low = 0;
  let high = elements.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    const element = elements[middle];
    if (element && element.getBoundingClientRect().bottom > 0) high = middle;
    else low = middle + 1;
  }
  return elements[low] ?? undefined;
}

/**
 * The block at the top of the view, which is the innermost element crossing the top, or the
 * first block below it when the top falls between two. It can be an element of raw HTML, with no
 * line number. Undefined at the start of the document, above its first block.
 */
function readingBlock(view: HTMLElement): Element | undefined {
  const first = view.firstElementChild;
  if (!first || first.getBoundingClientRect().top >= 0) return undefined;
  let block: Element | undefined;
  let element = firstNotAboveTop(view.children);
  while (element) {
    block = element;
    // Only a block that crosses the top has a block nearer the top inside it. An element with no
    // line number, such as a code block's wrapper, is looked into for the block it holds.
    const hasLine = startLine(element) !== undefined;
    if (hasLine && element.getBoundingClientRect().top >= 0) break;
    element = firstNotAboveTop(element.children);
  }
  return block;
}

/**
 * The reading position, as Reload keeps it: the line number of the block at the top of the view,
 * or of the innermost block with one around it. Undefined at the start of the document.
 */
export function readingPosition(view: HTMLElement): number | undefined {
  const block = readingBlock(view)?.closest("[data-sourcepos]");
  return block && view.contains(block) ? startLine(block) : undefined;
}

/**
 * The block to put at the top of the view for the reading position `line`: the innermost one
 * that starts on it, or else the one that starts last before it. Footnotes come last in the view
 * but not in the file, so this goes by the line rather than by the order in the view.
 */
function blockAt(view: HTMLElement, line: number): Element | undefined {
  let best: Element | undefined;
  let bestLine = 0;
  for (const block of view.querySelectorAll("[data-sourcepos]")) {
    const start = startLine(block);
    // On a tie the later one is inside the earlier, and so the innermost.
    if (start !== undefined && start <= line && start >= bestLine) {
      best = block;
      bestLine = start;
    }
  }
  return best;
}

/**
 * Puts the view back at the reading position `line`, as `scrollToTop` does. Returns false if
 * `stillWanted` said no after the frame; a line with no block before it leaves the view where it
 * is and returns true.
 */
export async function returnToReadingPosition(
  view: HTMLElement,
  line: number,
  stillWanted: () => boolean,
): Promise<boolean> {
  const block = blockAt(view, line);
  return block ? scrollToTop(block, stillWanted) : true;
}

/**
 * Whether a node of the view is at or after the reading position: in the block at the top of the
 * view, or after it in the document. At the start of the document every node is. It compares
 * places in the document rather than line numbers, which don't rise in document order: footnotes
 * come last but keep their definitions' lines, and raw HTML has none.
 */
export function atOrAfterReadingPosition(
  view: HTMLElement,
): (node: Node) => boolean {
  const block = readingBlock(view);
  if (block === undefined) return () => true;
  return (node) =>
    block.contains(node) ||
    (block.compareDocumentPosition(node) & Node.DOCUMENT_POSITION_FOLLOWING) !==
      0;
}
