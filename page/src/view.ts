import { invoke } from "@tauri-apps/api/core";
import { addCopyButtons } from "./copy";
import { listenToApp } from "./events";
import { showNotice } from "./notice";
import { open } from "./open";
import {
  afterNextFrame,
  readingPosition,
  returnToReadingPosition,
  scrollToTop,
} from "./reading-position";
import { epochMs, reportFirstPaint, reportReload } from "./timing";

/** Selects the view that shows the rendered document, and none of the page's own elements. */
export const DOCUMENT_VIEW_SELECTOR = "article.markdown-body";

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
  await scrollToTop(target, () => target.isConnected);
}

/**
 * The app's answer when the page asks for its document (`DocumentToShow` in `app/src/render.rs`):
 * the rendered document as its HTML, and the heading to show it at, from a link like `guide.md#install`,
 * or null for its top. Null while the window shows no document.
 */
type DocumentToShow = { document: string; heading: string | null } | null;

type ShownListener = (view: HTMLElement) => void;

const shownListeners = new Set<ShownListener>();

/**
 * Runs `listener` with the view each time the view's content has been replaced: when the window
 * shows its document or another one, after a reload once the view is back at the reading
 * position, and when the window shows no document or why it can't show one. A request that
 * replaced the content but was then overtaken leaves the call to the request that overtook it,
 * which makes it even if it fails, so a replaced view is always heard of, once the last request
 * ends. This is the one hook for that, so Find recounts here (ticket 14). Returns a function that
 * removes the listener.
 */
export function whenDocumentShown(listener: ShownListener): () => void {
  shownListeners.add(listener);
  return () => {
    shownListeners.delete(listener);
  };
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
  /** Whether the view's content changed since the `whenDocumentShown` listeners last ran. */
  let unannounced = false;

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
    unannounced = true;
  };

  /**
   * Runs the `whenDocumentShown` listeners if the view's content changed since they last ran.
   * Each request that is still the latest when it ends calls it, however it ends, so content
   * placed by a request that a later one overtook is announced by the later one.
   */
  const announce = () => {
    if (!unannounced) return;
    unannounced = false;
    for (const listener of shownListeners) listener(view);
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
      if (shown === null) {
        announce();
        return undefined;
      }
      scrollTo(0, 0);
      if (shown.heading !== null) {
        await scrollToHeading(shown.heading);
        if (request !== latestRequest) return undefined;
      }
      announce();
      return received;
    } catch (error) {
      if (request !== latestRequest) return undefined;
      place(document.createTextNode(String(error)));
      announce();
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
      if (request !== latestRequest) return;
      showNotice(String(error));
      // The last render stays, but a reload this one overtook may have put it in place.
      announce();
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
    const line = readingPosition(view);
    place(parsed.content);
    const inserted = epochMs();
    if (
      line !== undefined &&
      !(await returnToReadingPosition(
        view,
        line,
        () => request === latestRequest,
      ))
    )
      return;
    const anchored = epochMs();
    announce();
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
