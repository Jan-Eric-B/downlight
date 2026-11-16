import {
  askApp,
  listenToApp,
  type DocumentFile,
  type DocumentToShow,
  type Notice,
  type Step,
  type Steps,
} from "./app";
import { addCopyButtons } from "./copy";
import { WindowHistory, type Line } from "./history";
import { noticeContent, showingWhy, showNotice } from "./notice";
import { open } from "./open";
import {
  afterNextFrame,
  openSectionsAround,
  readingPosition,
  returnToReadingPosition,
  scrollToTop,
  type ScrolledView,
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
 * Selects the element the document scrolls in, below the title bar, so the scroll bar starts below
 * the bar too (window design ticket 01). `scripts/measure.js` scrolls it by the same selector.
 */
export const SCROLLER_SELECTOR = ".view-scroller";

/**
 * The element of the shown document whose id is `heading`, or else the anchor with that name, as
 * a browser finds a URL's fragment. Only the document is searched, never the page's own elements.
 */
function headingTarget(heading: string): Element | undefined {
  const view = document.querySelector(DOCUMENT_VIEW_SELECTOR);
  if (!view) return undefined;
  // By id and by name rather than by a selector, which would need the heading escaped; the
  // page's own elements have no ids or names.
  const byId = document.getElementById(heading);
  return byId && view.contains(byId)
    ? byId
    : [...document.getElementsByName(heading)].find(
        (element) => element.localName === "a" && view.contains(element),
      );
}

/**
 * Scrolls the shown document to `heading` (`headingTarget`); null is the top. A heading in a
 * closed section opens it, as Find does. A heading that isn't there leaves the view where it is.
 */
async function scrollToHeading(heading: string | null): Promise<void> {
  if (heading === null) {
    scrollDocumentToTop();
    return;
  }
  const target = headingTarget(heading);
  if (!target) return;
  openSectionsAround(target);
  await scrollToTop(target, () => target.isConnected);
}

/**
 * Scrolls the document to its top. The view makes its scroller when it starts, before anything
 * can ask for a scroll, so a missing one is a page bug, and it fails rather than doing nothing.
 */
function scrollDocumentToTop(): void {
  const scroller = document.querySelector(SCROLLER_SELECTOR);
  if (!scroller) throw new Error("The document view hasn't started.");
  scroller.scrollTo(0, 0);
}

/**
 * Why the view's content was replaced, as the `whenDocumentShown` listeners hear it: `shown` when
 * the window shows its document or another one, the empty state or why it can't show one,
 * `reload` when it shows the document's new render at the reading position, and `return` when
 * Back or Forward gave the window a document it showed before, or why it can't show it now.
 */
export type ShownReason = "shown" | "reload" | "return";

/** What the `whenDocumentShown` listeners hear each time the view's content was replaced. */
interface Shown {
  /** The view and its scroller. */
  view: ScrolledView;
  reason: ShownReason;
  /**
   * The file of the document the window shows: null while it shows none, or when the app couldn't
   * be asked.
   */
  file: DocumentFile | null;
}

type ShownListener = (shown: Shown) => void;

const shownListeners = new Set<ShownListener>();

/**
 * Runs `listener` with the view and its scroller, why the view's content changed, and the file of
 * the document the window shows, each time the view's content has been replaced: when the window shows its document or another
 * one, after a reload once the view is back at the reading position, and when the window shows no
 * document or why it can't show one. A request that replaced the content but was then overtaken
 * leaves the call to the request that overtook it, which makes it even if it fails, so a replaced
 * view is always heard of, once the last request ends. This is the one hook for that, so Find
 * recounts here (ticket 14) and the title bar names the file (window design ticket 04). Returns a
 * function that removes the listener.
 */
export function whenDocumentShown(listener: ShownListener): () => void {
  shownListeners.add(listener);
  return () => {
    shownListeners.delete(listener);
  };
}

/** What the rest of the page asks of the started view. */
interface StartedView {
  request: () => Promise<void>;
  showing: () => number | null;
  step: (step: Step) => void;
  canStep: (step: Step) => boolean;
  visitHeading: (heading: string | null) => Promise<void>;
}

/** The started view, until a view starts. */
let theView: StartedView | undefined;

/**
 * Asks the app for the window's document and places the answer as it says, as the app's
 * `ask-for-document` message makes the view do, and as F5 does (ticket 26). Resolves once the
 * answer is in place, or once a later request overtook it. Does nothing until the view has
 * started.
 */
export function requestDocument(): Promise<void> {
  return theView?.request() ?? Promise.resolve();
}

/**
 * The showing whose render the view shows, which a link in it names (ticket 26), or null while
 * it shows the empty state, an error or nothing yet, or hasn't started.
 */
export function showingInView(): number | null {
  return theView?.showing() ?? null;
}

/**
 * Goes back or forward in the window's history (ticket 12), as Alt+Left and Alt+Right, the
 * mouse's side buttons and the right-click menu do: to the heading visited before or after in the
 * document in view, back at the reading position the view had there, or else to the document the
 * window showed before or after, which the app gives the window as any other. Nowhere to go does
 * nothing. Does nothing until the view has started.
 */
export function stepThroughHistory(step: Step): void {
  theView?.step(step);
}

/** Whether `step` goes anywhere now, for the right-click menu. */
export function canStepThroughHistory(step: Step): boolean {
  return theView?.canStep(step) ?? false;
}

/**
 * Scrolls the shown document to `heading`, as a link does, or to its top for null: one more step
 * in the window's history, which Back returns from to where the view was. A heading the document
 * doesn't have leaves the view, and the history, as they are.
 */
export function visitHeading(heading: string | null): Promise<void> {
  return theView?.visitHeading(heading) ?? Promise.resolve();
}

/**
 * The reader's state in the view, read just before its content is swapped and landed with just
 * after. Ticks (ticket 13) belong here.
 */
interface ReaderState {
  /** The reading position, or undefined at the start of the document. */
  line: Line;
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
    scrollDocumentToTop();
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

/**
 * Puts the view at the reading position `line`, or at the top for undefined. Returns false if
 * `stillWanted` said no after the frame.
 */
function landAtLine(
  view: HTMLElement,
  line: Line,
  stillWanted: () => boolean,
): Promise<boolean> {
  // From the top, so a line with no block before it shows the top rather than where the view was.
  scrollDocumentToTop();
  return line === undefined
    ? Promise.resolve(true)
    : returnToReadingPosition(view, line, stillWanted);
}

/** Lands where the view was in `showing` when it last left it, as `history` keeps it. */
function atReturn(history: WindowHistory, showing: number): Landing {
  return (view, _before, isLatest) =>
    landAtLine(view, history.returnTo(showing), isLatest);
}

/** What a request puts in the view, and how it lands there. */
interface Placement {
  reason: ShownReason;
  /** The showing whose render this is, or null for the empty state or an error. */
  showing: number | null;
  /**
   * The file of the document the window shows, also when it can't be read, or null for the
   * empty state or when the app couldn't be asked.
   */
  file: DocumentFile | null;
  /** The core's rendered HTML, why the document can't be shown, or null for the empty state. */
  content: { html: string } | { why: Notice } | null;
  land: Landing;
  /**
   * Whether the app has a document to go back or forward to, or null when the app couldn't be
   * asked.
   */
  steps: Steps | null;
}

/** Nowhere to go, as for a window that shows no document. */
const NO_STEPS: Steps = { back: false, forward: false };

/** What a request does when the view keeps what it shows: tell the reader why. */
interface Telling {
  notice: Notice;
}

/**
 * Shows why the window can't show its document from `file`, in place of any render, for a show
 * or a return (`reason`), with what the app said of its history in `steps`.
 */
function placeError(
  why: Notice,
  file: DocumentFile | null,
  reason: ShownReason,
  steps: Steps | null,
): Placement {
  return {
    reason,
    showing: null,
    file,
    content: { why },
    land: atTop,
    steps,
  };
}

/**
 * What to do with the app's answer: show the document at its heading or top, return to where the
 * view was in it, as `history` keeps it, or reload it at the reading position. A reload that can't
 * read the file keeps the last render and says why, so the reader can keep reading.
 */
function placementFor(
  answer: DocumentToShow,
  history: WindowHistory,
): Placement | Telling {
  if (answer === null)
    return {
      reason: "shown",
      showing: null,
      file: null,
      content: null,
      land: atTop,
      steps: NO_STEPS,
    };
  const { showing, file, steps } = answer;
  switch (answer.place) {
    case "reload":
      if ("error" in answer) return { notice: answer.error };
      return {
        reason: "reload",
        showing,
        file,
        content: { html: answer.document },
        land: atReadingPosition,
        steps,
      };
    case "return":
      if ("error" in answer)
        return placeError(answer.error, file, "return", steps);
      return {
        reason: "return",
        showing,
        file,
        content: { html: answer.document },
        land: atReturn(history, showing),
        steps,
      };
    case "show":
      if ("error" in answer)
        return placeError(answer.error, file, "shown", steps);
      return {
        reason: "shown",
        showing,
        file,
        content: { html: answer.document },
        land: atHeading(answer.heading),
        steps,
      };
  }
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
  button.addEventListener("click", () => {
    open();
  });
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

/** The element the document scrolls in, which `SCROLLER_SELECTOR` selects. */
function createScroller(): HTMLElement {
  const scroller = document.createElement("div");
  scroller.className = SCROLLER_SELECTOR.slice(1);
  scroller.tabIndex = -1;
  return scroller;
}

/**
 * Keeps the keys that scroll (arrows, Page Up and Down, Space) on the document. The page's root
 * no longer scrolls, so they scroll `scroller` only while it or something in it has the focus: it
 * takes the focus now, and again whenever the focus falls back to the page's body, as when Esc
 * closes Find.
 */
function keepTheKeysOn(scroller: HTMLElement): void {
  const takeTheKeys = () => {
    scroller.focus({ preventScroll: true });
  };
  takeTheKeys();
  document.addEventListener("focusout", () => {
    // After the focus has moved on, so a focus that moves to another element isn't taken back.
    setTimeout(() => {
      const focused = document.activeElement;
      if (focused === null || focused === document.body) takeTheKeys();
    }, 0);
  });
}

/**
 * Shows the window's document, or the empty state while it has none, and asks for it again
 * whenever the app says the window shows another document or the document's file changed. The
 * app's answer says whether to show it, at its heading or top, reload it at the same reading
 * position (ticket 23), or return to where the view was in it after Back or Forward (ticket 12);
 * the page never decides that by which message made it ask. When the app says the file of the
 * showing in view is gone, it says so over the last render (ticket 26). `scriptStart` is when the
 * page's script started, for the first paint report.
 */
export async function startDocumentView(scriptStart: number): Promise<void> {
  markFirstPaint("scriptStart", scriptStart);
  const goneBar = createGoneBar();
  const view = document.createElement("article");
  view.className = "markdown-body";
  const empty = createEmptyState();
  empty.hidden = true;
  const scroller = createScroller();
  scroller.append(goneBar, view, empty);
  document.body.append(scroller);
  keepTheKeysOn(scroller);
  const shown: ScrolledView = { view, scroller };

  let latestRequest = 0;
  const history = new WindowHistory();
  /**
   * The showing whose render the view shows, sent with each request so the app can tell a show
   * from a reload; null while it shows the empty state, an error or nothing yet.
   */
  let showing: number | null = null;
  /** The file of the document the view shows, for the `whenDocumentShown` listeners. */
  let file: DocumentFile | null = null;
  /**
   * The showing whose file the app said went away since the window last asked for its render, or
   * null. The bar shows only while the view shows that showing (ticket 26), so a `gone` for a
   * document the window has left never shows it, and one for the document the window was just
   * given shows it once that document is in view.
   */
  let goneShowing: number | null = null;
  const showGoneBar = () => {
    goneBar.hidden = showing === null || showing !== goneShowing;
  };
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
  const ask = async (): Promise<Placement | Telling> => {
    let answer: DocumentToShow;
    try {
      answer = await askApp("render_document", { showing });
    } catch (error) {
      // The app couldn't be asked at all, so there is no answer to say what to do: a view that
      // shows a document keeps it, as a failed reload does.
      return showing === null
        ? placeError(String(error), null, "shown", null)
        : { notice: String(error) };
    }
    // Shown as soon as it comes, even if a later request overtakes this one: the app hands a held
    // notice over once, with the first answer (ticket 26).
    if (answer?.notice) showNotice(answer.notice);
    return placementFor(answer, history);
  };

  /**
   * Replaces the view's content with `content`, or shows the empty state for null. The history
   * learns of the swap here, the one place every request's content goes in, with the reader's
   * state from just before it.
   */
  const replaceContent = (
    placement: Placement,
    content: Node | null,
    before: ReaderState,
  ) => {
    const left = showing;
    if (left !== null && left !== placement.showing)
      history.leave(left, before.line);
    if (placement.reason === "shown") history.showAnew(placement.showing, left);
    if (placement.steps) history.appCanStep(placement.steps);
    showing = placement.showing;
    file = placement.file;
    view.hidden = content === null;
    empty.hidden = content !== null;
    showGoneBar();
    if (content === null) view.replaceChildren();
    else view.replaceChildren(content);
    // Listeners that haven't heard of a show or a return yet hear it, even if a reload came after
    // it.
    if (unannounced === undefined || placement.reason !== "reload")
      unannounced = placement.reason;
  };

  /** Counts the steps within a document, so that only the latest one lands. */
  let latestStepWithin = 0;

  /**
   * Back or Forward (`stepThroughHistory`): within the showing in view if it has a visit that
   * way, or else through the app.
   */
  const step = (to: Step) => {
    if (showing !== null) {
      const within = history.stepWithin(showing, to, readingPosition(shown));
      if (within) {
        const inView = showing;
        const thisStep = ++latestStepWithin;
        void landAtLine(
          view,
          within.line,
          () => showing === inView && thisStep === latestStepWithin,
        );
        return;
      }
    }
    showingWhy(askApp("step_history", { step: to }));
  };

  /** A link's scroll to a heading, as one more visit (`visitHeading`). */
  const visitHeadingInView = async (heading: string | null) => {
    if (showing === null) return;
    if (heading !== null && !headingTarget(heading)) return;
    if (history.visitHeading(showing, readingPosition(shown)))
      showingWhy(askApp("forget_forward", { showing }));
    await scrollToHeading(heading);
  };

  /**
   * Puts the answer in the view and lands it there, or shows its notice. After each wait it
   * stops if a later request overtook this one.
   */
  const place = async (
    outcome: Placement | Telling,
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
          : noticeContent(given.why);
    if (outcome.reason === "reload") {
      // Parsed in a task and a frame of its own, apart from the swap and the layout for the
      // anchoring, so that neither holds up the page for over 100 ms with a 1 MB document
      // (ADR 0003). A show doesn't wait for the frame, which would hold up first paint.
      await afterNextFrame();
      if (!isLatest()) return;
    }
    // Read just before the swap, so a scroll while the new render was parsed counts.
    const before: ReaderState = { line: readingPosition(shown) };
    replaceContent(outcome, content, before);
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
    for (const listener of shownListeners)
      listener({ view: shown, reason, file });
  };

  /**
   * The one way the view gets its document: asks for it, places the answer and lands it, unless
   * a later request overtakes it on the way, so the window ends up with what it was given last.
   */
  const request = async (): Promise<void> => {
    // The file is read for this request, so only a "gone" after this keeps the bar. A reload that
    // can't read the file leaves the bar as it is.
    goneShowing = null;
    const thisRequest = ++latestRequest;
    const isLatest = () => thisRequest === latestRequest;
    const outcome = await ask();
    if (isLatest()) await place(outcome, isLatest);
    if (isLatest()) settle();
  };
  theView = {
    request,
    showing: () => showing,
    step,
    canStep: (to) => history.canStep(showing, to),
    visitHeading: visitHeadingInView,
  };

  // The listeners only start: waiting for them would hold up first paint.
  listenToApp("ask-for-document", () => {
    void request();
  });
  listenToApp("scroll-to-heading", (heading) => {
    void visitHeadingInView(heading);
  });
  listenToApp("document-gone", (gone) => {
    goneShowing = gone;
    showGoneBar();
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
