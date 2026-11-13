import { askApp, listenToApp, type DocumentToShow } from "./app";
import { addCopyButtons } from "./copy";
import { showNotice } from "./notice";
import { open } from "./open";
import {
  afterNextFrame,
  openSectionsAround,
  readingPosition,
  returnToReadingPosition,
  scrollToTop,
} from "./reading-position";
import {
  epochMs,
  markFirstPaint,
  reportFirstPaint,
  reportReload,
} from "./timing";

/** Selects the view that shows the rendered document, and none of the page's own elements. */
export const DOCUMENT_VIEW_SELECTOR = "article.markdown-body";

/**
 * Scrolls the shown document to the element whose id is `heading`, or else to the anchor with
 * that name, as a browser does for a URL's fragment; null is the top. A heading in a closed
 * section opens it, as Find does. Only the document is searched, never the page's own elements. A
 * heading that isn't there leaves the view where it is.
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
  openSectionsAround(target);
  await scrollToTop(target, () => target.isConnected);
}

/**
 * Why the view's content was replaced, as the `whenDocumentShown` listeners hear it: `shown` when
 * the window shows its document or another one, the empty state or why it can't show one, and
 * `reload` when it shows the document's new render at the reading position.
 */
export type ShownReason = "shown" | "reload";

type ShownListener = (view: HTMLElement, reason: ShownReason) => void;

const shownListeners = new Set<ShownListener>();

/**
 * Runs `listener` with the view, and why its content changed, each time the view's content has
 * been replaced: when the window shows its document or another one, after a reload once the view
 * is back at the reading position, and when the window shows no document or why it can't show
 * one. A request that replaced the content but was then overtaken leaves the call to the request
 * that overtook it, which makes it even if it fails, so a replaced view is always heard of, once
 * the last request ends. This is the one hook for that, so Find recounts here (ticket 14). Returns
 * a function that removes the listener.
 */
export function whenDocumentShown(listener: ShownListener): () => void {
  shownListeners.add(listener);
  return () => {
    shownListeners.delete(listener);
  };
}

/** The started view's request, until a view starts. */
let requestOfTheView: (() => Promise<void>) | undefined;

/**
 * Asks the app for the window's document and places the answer as it says, as the app's messages
 * make the view do; F5 is to call it (ticket 26). Resolves once the answer is in place, or once a
 * later request overtook it. Does nothing until the view has started.
 */
export function requestDocument(): Promise<void> {
  return requestOfTheView?.() ?? Promise.resolve();
}

/**
 * The reader's state in the view, read just before its content is swapped and landed with just
 * after. Ticks (ticket 13) and a history entry's line (ticket 12) belong here.
 */
interface ReaderState {
  /** The reading position, or undefined at the start of the document. */
  line: number | undefined;
}

/**
 * Puts the view where a request lands once its content is in place, given the reader's state
 * from before the swap. Returns false if a later request overtook it on the way.
 */
type Landing = (
  view: HTMLElement,
  before: ReaderState,
  isLatest: () => boolean,
) => Promise<boolean>;

/** Lands at the heading a link named, as `scrollToHeading` finds it, or at the top for null. */
function atHeading(heading: string | null): Landing {
  return async (_view, _before, isLatest) => {
    scrollTo(0, 0);
    if (heading === null) return true;
    await scrollToHeading(heading);
    return isLatest();
  };
}

const atTop = atHeading(null);

/** Lands back at the reading position from before the swap. */
const atReadingPosition: Landing = (view, { line }, isLatest) =>
  line === undefined
    ? Promise.resolve(true)
    : returnToReadingPosition(view, line, isLatest);

/** What a request puts in the view, and how it lands there. */
interface Placement {
  reason: ShownReason;
  /** The showing whose render this is, or null for the empty state or an error. */
  showing: number | null;
  /** The core's rendered HTML, why the document can't be shown, or null for the empty state. */
  content: { html: string } | { text: string } | null;
  land: Landing;
}

/** What a request does when the view keeps what it shows: tell the reader why. */
interface Notice {
  notice: string;
}

/** Shows why the window can't show its document, in place of any render. */
function placeError(why: string): Placement {
  return {
    reason: "shown",
    showing: null,
    content: { text: why },
    land: atTop,
  };
}

/**
 * What to do with the app's answer: show the document at its heading or top, or reload it at the
 * reading position. A reload that can't read the file keeps the last render and says why, so the
 * reader can keep reading.
 */
function placementFor(answer: DocumentToShow): Placement | Notice {
  if (answer === null)
    return { reason: "shown", showing: null, content: null, land: atTop };
  if (answer.place === "reload") {
    if ("error" in answer) return { notice: answer.error };
    return {
      reason: "reload",
      showing: answer.showing,
      content: { html: answer.document },
      land: atReadingPosition,
    };
  }
  if ("error" in answer) return placeError(answer.error);
  return {
    reason: "shown",
    showing: answer.showing,
    content: { html: answer.document },
    land: atHeading(answer.heading),
  };
}

/**
 * Parses the core's rendered HTML into the view's next content and prepares it as every render
 * is, with copy buttons on its code blocks (ticket 07). The template's content is inert: its
 * images load only once it is in the view.
 */
function prepare(html: string): DocumentFragment {
  const parsed = document.createElement("template");
  // The HTML comes only from the core's render, never from the page.
  parsed.innerHTML = html;
  addCopyButtons(parsed.content);
  return parsed.content;
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
  markFirstPaint("scriptStart", scriptStart);
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
  /**
   * Why the view's content changed since the `whenDocumentShown` listeners last ran, or
   * undefined if it didn't.
   */
  let unannounced: ShownReason | undefined;
  let settleFirst: (showsDocument: boolean) => void = () => undefined;
  /**
   * Resolves once a request that is still the latest when it ends has ended, with whether the
   * view then shows a document.
   */
  const firstSettled = new Promise<boolean>((resolve) => {
    settleFirst = resolve;
  });

  /**
   * Asks the app for the window's document, sending the showing the view shows, and says what to
   * do with the answer.
   */
  const ask = async (): Promise<Placement | Notice> => {
    let answer: DocumentToShow;
    try {
      answer = await askApp("render_document", { showing });
    } catch (error) {
      // The app couldn't be asked at all, so there is no answer to say what to do: a view that
      // shows a document keeps it, as a failed reload does.
      return showing === null
        ? placeError(String(error))
        : { notice: String(error) };
    }
    return placementFor(answer);
  };

  /** Replaces the view's content with `content`, or shows the empty state for null. */
  const replaceContent = (placement: Placement, content: Node | null) => {
    // A "gone" that came while a show was on the way was for the document the window has left,
    // as the app sends it after `document-shown` at times (ticket 23). A reload keeps the bar.
    if (placement.reason === "shown") fileGone = false;
    showing = placement.showing;
    view.hidden = content === null;
    empty.hidden = content !== null;
    goneBar.hidden = !fileGone;
    if (content === null) view.replaceChildren();
    else view.replaceChildren(content);
    // Listeners that haven't heard of a show yet hear "shown", even if a reload came after it.
    unannounced = unannounced === "shown" ? "shown" : placement.reason;
  };

  /**
   * Puts the answer in the view and lands it there, or shows its notice. After each wait it
   * stops if a later request overtook this one.
   */
  const place = async (
    outcome: Placement | Notice,
    isLatest: () => boolean,
  ): Promise<void> => {
    if ("notice" in outcome) {
      showNotice(outcome.notice);
      return;
    }
    const received = epochMs();
    const { content: given } = outcome;
    const content =
      given === null
        ? null
        : "html" in given
          ? prepare(given.html)
          : document.createTextNode(given.text);
    if (outcome.reason === "reload") {
      // Parsed in a task and a frame of its own, apart from the swap and the layout for the
      // anchoring, so that neither holds up the page for over 100 ms with a 1 MB document
      // (ADR 0003). A show doesn't wait for the frame, which would hold up first paint.
      await afterNextFrame();
      if (!isLatest()) return;
    }
    // Read just before the swap, so a scroll while the new render was parsed counts.
    const before: ReaderState = { line: readingPosition(view) };
    replaceContent(outcome, content);
    const inserted = epochMs();
    if (outcome.reason === "shown") {
      markFirstPaint("documentReceived", received);
      markFirstPaint("documentInserted", inserted);
    }
    if (!(await outcome.land(view, before, isLatest))) return;
    if (outcome.reason === "reload")
      reportReload({ received, inserted, anchored: epochMs() });
  };

  /**
   * Ends the latest request, however it ended: runs the `whenDocumentShown` listeners if the
   * view's content changed since they last ran, so content placed by a request that a later one
   * overtook is announced by the later one.
   */
  const settle = () => {
    settleFirst(showing !== null);
    if (unannounced === undefined) return;
    const reason = unannounced;
    unannounced = undefined;
    for (const listener of shownListeners) listener(view, reason);
  };

  /**
   * The one way the view gets its document: asks for it, places the answer and lands it, unless
   * a later request overtakes it on the way, so the window ends up with what it was given last.
   */
  const request = async (): Promise<void> => {
    // The file is read for this request, so only a "gone" after this keeps the bar. A reload that
    // can't read the file leaves the bar as it is.
    fileGone = false;
    const thisRequest = ++latestRequest;
    const isLatest = () => thisRequest === latestRequest;
    const outcome = await ask();
    if (isLatest()) await place(outcome, isLatest);
    if (isLatest()) settle();
  };
  requestOfTheView = request;

  // The listeners only start: waiting for them would hold up first paint.
  listenToApp("document-shown", () => {
    void request();
  });
  listenToApp("scroll-to-heading", (heading) => {
    void scrollToHeading(heading);
  });
  listenToApp("document-changed", (change) => {
    if (change === "gone") {
      fileGone = true;
      goneBar.hidden = false;
    }
    if (change === "reload") void request();
  });
  listenToApp("notice", showNotice);

  markFirstPaint("documentRequested");
  void request();
  // Waits for the latest request, not the first one, which a save can overtake.
  const showsDocument = await firstSettled;
  // The window opens hidden and shows once the frame with its document, the empty state or the
  // error is drawn, so it never flashes white or shows an empty frame (ticket 04).
  await afterNextFrame();
  markFirstPaint("firstFrame");
  // If this fails, the app shows the window after a few seconds anyway.
  askApp("show_window").catch(() => undefined);
  if (showsDocument) reportFirstPaint();
}
