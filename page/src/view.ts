import { invoke } from "@tauri-apps/api/core";
import { open } from "./commands";
import { addCopyButtons } from "./copy";
import { listenToApp } from "./events";
import { showNotice } from "./notice";
import { epochMs, reportFirstPaint, reportReload } from "./timing";

/**
 * Resolves once the next frame is done: the task queued from `requestAnimationFrame` runs after
 * that frame's style, layout and paint.
 */
function afterNextFrame(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      setTimeout(resolve, 0);
    });
  });
}

/** Selects the view that shows the rendered document, and none of the page's own elements. */
export const DOCUMENT_VIEW_SELECTOR = "article.markdown-body";

/**
 * Scrolls `element` to the top of the view, and once more after the next frame if
 * `stillCurrent` says the scroll is still wanted then. Returns whether it was.
 */
async function putAtTop(
  element: Element,
  stillCurrent: () => boolean,
): Promise<boolean> {
  element.scrollIntoView({ block: "start" });
  // The blocks above it had estimated heights until this frame laid them out (ADR 0003), so it
  // may have moved.
  await afterNextFrame();
  if (!stillCurrent()) return false;
  element.scrollIntoView({ block: "start" });
  return true;
}

/**
 * Scrolls the shown document to the element whose id is `heading`, or else to the anchor with
 * that name, as a browser does for a URL's fragment; null is the top. Only the document is
 * searched, never the page's own elements. A heading that isn't there leaves the view where it
 * is.
 */
export async function scrollToHeading(heading: string | null): Promise<void> {
  if (heading === null) {
    scrollTo(0, 0);
    return;
  }
  const view = document.querySelector(DOCUMENT_VIEW_SELECTOR);
  if (!view) return;
  // By id and by name rather than by a selector, which would need the heading escaped; the
  // page's own elements have no ids or names.
  const byId = document.getElementById(heading);
  const target =
    byId && view.contains(byId)
      ? byId
      : [...document.getElementsByName(heading)].find(
          (element) => element.localName === "a" && view.contains(element),
        );
  if (!target) return;
  await putAtTop(target, () => target.isConnected);
}

/**
 * The app's answer when the page asks for its document (`DocumentToShow` in `app/src/render.rs`):
 * the rendered document as its HTML, and the heading to show it at, from a link like `guide.md#install`,
 * or null for its top. Null while the window shows no document.
 */
type DocumentToShow = { document: string; heading: string | null } | null;

const shownListeners = new Set<() => void>();

/**
 * Runs `listener` each time a rendered document is in place in the view: when the window shows
 * its document or another one, and after a reload, once the view is back at the reading
 * position. This is the one hook for that, so Find recounts here (ticket 14). Returns a function
 * that removes the listener.
 */
export function whenDocumentShown(listener: () => void): () => void {
  shownListeners.add(listener);
  return () => {
    shownListeners.delete(listener);
  };
}

function documentShown(): void {
  for (const listener of shownListeners) listener();
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
 * The reading position: the line number of the block at the top of the view, which is the
 * innermost block crossing the top, or the first block below it when the top falls between two.
 * Undefined at the start of the document, above its first block.
 */
function readingLine(view: HTMLElement): number | undefined {
  const first = view.firstElementChild;
  if (!first || first.getBoundingClientRect().top >= 0) return undefined;
  let line: number | undefined;
  let element = firstNotAboveTop(view.children);
  while (element) {
    const start = startLine(element);
    line = start ?? line;
    // Only a block that crosses the top has a block nearer the top inside it. An element with no
    // line number, such as a code block's wrapper, is looked into for the block it holds.
    if (start !== undefined && element.getBoundingClientRect().top >= 0) break;
    element = firstNotAboveTop(element.children);
  }
  return line;
}

/**
 * The block to put at the top of the view for the reading line `line`: the innermost one that
 * starts on it, or else the one that starts last before it. Footnotes come last in the view but
 * not in the file, so this goes by the line rather than by the order in the view.
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

/** What a window without a document shows: a way to open one. */
function createEmptyState(): HTMLElement {
  const empty = document.createElement("main");
  empty.className = "empty-window markdown-body";
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = "Open file";
  button.addEventListener("click", () => void open());
  const hint = document.createElement("p");
  hint.textContent = "or drop a file here";
  empty.append(button, hint);
  return empty;
}

/** The bar over the last render that says its file is gone. */
function createGoneBar(): HTMLElement {
  const bar = document.createElement("div");
  // A markdown-body, for github-markdown-css's font and its light or dark colors.
  bar.className = "file-gone markdown-body";
  bar.setAttribute("role", "status");
  bar.textContent = "File deleted or moved";
  bar.hidden = true;
  return bar;
}

/**
 * Shows the window's document, or the empty state while it has none, and shows the next one
 * whenever the app says the window shows another document. When the app says the document's file
 * changed, it shows the reload at the same reading position, or says the file is gone.
 * `scriptStart` is when the page's script started, for the first paint report.
 */
export async function startDocumentView(scriptStart: number): Promise<void> {
  const goneBar = createGoneBar();
  const view = document.createElement("article");
  view.className = "markdown-body";
  const empty = createEmptyState();
  empty.hidden = true;
  document.body.append(goneBar, view, empty);

  let latestRequest = 0;
  /** Whether the document's file went away since the window last asked for its render. */
  let fileGone = false;

  /**
   * Shows `content` in the view: the core's rendered HTML, a render already parsed, or an error's
   * text. Null shows the empty state instead.
   */
  const place = (content: string | Node | null) => {
    view.hidden = content === null;
    empty.hidden = content !== null;
    goneBar.hidden = !fileGone;
    if (content instanceof Node) view.replaceChildren(content);
    // The HTML comes only from the core's render, never from the page.
    else view.innerHTML = content ?? "";
  };

  /**
   * Asks the app for the window's document and shows it, at its heading if a link named one and
   * otherwise at the top. Returns when the HTML arrived, if this request inserted a document; a
   * request that a later one overtook shows nothing, so the window ends up with the document it
   * was shown last.
   */
  const showDocument = async (): Promise<number | undefined> => {
    fileGone = false;
    const request = ++latestRequest;
    try {
      const shown = await invoke<DocumentToShow>("render_document");
      if (request !== latestRequest) return undefined;
      const received = epochMs();
      place(shown?.document ?? null);
      addCopyButtons(view);
      if (shown === null) return undefined;
      scrollTo(0, 0);
      if (shown.heading !== null) {
        await scrollToHeading(shown.heading);
        if (request !== latestRequest) return undefined;
      }
      documentShown();
      return received;
    } catch (error) {
      if (request !== latestRequest) return undefined;
      place(document.createTextNode(String(error)));
      return undefined;
    }
  };

  /**
   * Shows the document's new render at the reading position. A reload that can't read the file
   * keeps the last render and says why, so the reader can keep reading.
   */
  const reload = async () => {
    // The reload reads the file as it is now, so only a "gone" after this keeps the bar. A reload
    // that can't read the file leaves the bar as it is.
    fileGone = false;
    const request = ++latestRequest;
    let shown: DocumentToShow;
    try {
      shown = await invoke<DocumentToShow>("render_document");
    } catch (error) {
      if (request === latestRequest) showNotice(String(error));
      return;
    }
    if (request !== latestRequest) return;
    const received = epochMs();
    // Parsed in a task and a frame of its own, apart from the swap and the layout for the
    // anchoring, so that neither holds up the page for over 100 ms with a 1 MB document. The
    // template's content is inert: its images load only once it is in the view.
    const parsed = document.createElement("template");
    // The HTML comes only from the core's render, never from the page. The app reloads only a
    // window that shows a document, so the render is never null here. A heading the document was
    // shown at is left alone: a reload keeps the reading position.
    parsed.innerHTML = shown?.document ?? "";
    addCopyButtons(parsed.content);
    await afterNextFrame();
    if (request !== latestRequest) return;
    // Read now, so a scroll while the new render was parsed counts.
    const line = readingLine(view);
    place(parsed.content);
    const inserted = epochMs();
    const block = line === undefined ? undefined : blockAt(view, line);
    if (block && !(await putAtTop(block, () => request === latestRequest)))
      return;
    const anchored = epochMs();
    documentShown();
    reportReload({ received, inserted, anchored });
  };

  // The listeners only start: waiting for them would hold up first paint.
  listenToApp("document-shown", () => {
    void showDocument();
  });
  listenToApp("scroll-to-heading", (heading) => {
    void scrollToHeading(heading);
  });
  listenToApp("document-changed", (change) => {
    if (change === "gone") {
      fileGone = true;
      goneBar.hidden = false;
    }
    if (change === "reload") void reload();
  });
  listenToApp("notice", showNotice);

  const documentRequested = epochMs();
  const documentReceived = await showDocument();
  const documentInserted = epochMs();
  // The window opens hidden and shows once the frame with its document, the empty state or the
  // error is drawn, so it never flashes white or shows an empty frame (ticket 04).
  await afterNextFrame();
  const firstFrame = epochMs();
  // If this fails, the app shows the window after a few seconds anyway.
  invoke("show_window").catch(() => undefined);
  if (documentReceived !== undefined) {
    reportFirstPaint({
      scriptStart,
      documentRequested,
      documentReceived,
      documentInserted,
      firstFrame,
    });
  }
}
