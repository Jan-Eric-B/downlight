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
 * What the page does with a render, as the app decides it from the showing the page's view shows
 * (`Place` in `app/src/documents.rs`): show the document at the heading a link like
 * `guide.md#install` named, or at its top for null, or reload it at the reading position.
 */
export type Place =
  { place: "show"; heading: string | null } | { place: "reload" };

/**
 * What the page sends with `render_document`: the showing whose render its view shows, or null
 * while it shows the empty state, an error or nothing yet. Exported, with `DocumentToShow`, so the
 * page's tests answer as the app does in the shape the page reads.
 */
export type RenderDocumentArgs = { showing: number | null };

/**
 * The app's answer when the page asks for its document (`DocumentToShow` in
 * `app/src/documents.rs`), or null while the window shows no document: the showing the window is
 * at, for the next request, what to do with the render, and the rendered document as its HTML or
 * why it can't be read.
 */
export type DocumentToShow =
  | ({ showing: number } & Place & ({ document: string } | { error: string }))
  | null;

/** An answer that says to show the document. */
type ToShow = Extract<NonNullable<DocumentToShow>, { place: "show" }>;

/** An answer that says to reload the document. */
type ToReload = Extract<NonNullable<DocumentToShow>, { place: "reload" }>;

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
 * Shows the window's document, or the empty state while it has none, and asks for it again
 * whenever the app says the window shows another document or the document's file changed. The
 * app's answer says whether to show it, at its heading or top, or reload it at the same reading
 * position (ticket 23); the page never decides that by which message made it ask. When the app
 * says the file is gone, it says so over the last render. `scriptStart` is when the page's script
 * started, for the first paint report.
 */
export async function startDocumentView(scriptStart: number): Promise<void> {
  const goneBar = createGoneBar();
  const view = document.createElement("article");
  view.className = "markdown-body";
  const empty = createEmptyState();
  empty.hidden = true;
  document.body.append(goneBar, view, empty);

  let latestRequest = 0;
  /**
   * The showing whose render the view shows, sent with each request so the app can tell a show
   * from a reload; null while it shows the empty state, an error or nothing yet.
   */
  let showing: number | null = null;
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
   * Asks the app for the window's document, sending the showing the view shows, and shows or
   * reloads it as the answer says. Returns when the HTML arrived, if this request showed a
   * document; a request that a later one overtook does nothing more, so the window ends up with
   * what it was given last.
   */
  const requestDocument = async (): Promise<number | undefined> => {
    // The file is read for this request, so only a "gone" after this keeps the bar. A reload that
    // can't read the file leaves the bar as it is.
    fileGone = false;
    const request = ++latestRequest;
    let answer: DocumentToShow;
    const args: RenderDocumentArgs = { showing };
    try {
      answer = await invoke<DocumentToShow>("render_document", args);
    } catch (error) {
      // The app couldn't be asked at all, so there is no answer to say what to do: a view that
      // shows a document keeps it, as a failed reload does.
      if (request !== latestRequest) return undefined;
      if (showing === null) place(document.createTextNode(String(error)));
      else showNotice(String(error));
      announce();
      return undefined;
    }
    if (request !== latestRequest) return undefined;
    if (answer === null) {
      showing = null;
      place(null);
      announce();
      return undefined;
    }
    if (answer.place === "show") return show(answer, request);
    await reload(answer, request);
    return undefined;
  };

  /**
   * Shows a document the view didn't show, at its heading if a link named one and otherwise at
   * the top, or why it can't be read. Returns when the HTML arrived, if it showed the document.
   */
  const show = async (
    answer: ToShow,
    request: number,
  ): Promise<number | undefined> => {
    const received = epochMs();
    // A "gone" that came while this was on the way was for the document the window has left, as
    // the app sends it after `document-shown` at times (ticket 23); this one was just read.
    fileGone = false;
    if ("error" in answer) {
      showing = null;
      place(document.createTextNode(answer.error));
      announce();
      return undefined;
    }
    showing = answer.showing;
    place(answer.document);
    addCopyButtons(view);
    scrollTo(0, 0);
    if (answer.heading !== null) {
      await scrollToHeading(answer.heading);
      if (request !== latestRequest) return undefined;
    }
    announce();
    return received;
  };

  /**
   * Shows the document's new render at the reading position. A reload that can't read the file
   * keeps the last render and says why, so the reader can keep reading.
   */
  const reload = async (answer: ToReload, request: number) => {
    if ("error" in answer) {
      showNotice(answer.error);
      // The last render stays, but a reload this one overtook may have put it in place.
      announce();
      return;
    }
    const received = epochMs();
    // Parsed in a task and a frame of its own, apart from the swap and the layout for the
    // anchoring, so that neither holds up the page for over 100 ms with a 1 MB document. The
    // template's content is inert: its images load only once it is in the view.
    const parsed = document.createElement("template");
    // The HTML comes only from the core's render, never from the page.
    parsed.innerHTML = answer.document;
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
    void requestDocument();
  });
  listenToApp("scroll-to-heading", (heading) => {
    void scrollToHeading(heading);
  });
  listenToApp("document-changed", (change) => {
    if (change === "gone") {
      fileGone = true;
      goneBar.hidden = false;
    }
    if (change === "reload") void requestDocument();
  });
  listenToApp("notice", showNotice);

  const documentRequested = epochMs();
  const documentReceived = await requestDocument();
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
