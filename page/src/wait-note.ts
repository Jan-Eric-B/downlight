import stopGlyph from "@fluentui/svg-icons/icons/dismiss_16_regular.svg?raw";
import { askApp, type AnswerOf } from "./app";
import { iconButton } from "./icon-button";

/**
 * How long a request waits for the app's answer before the wait note says the document is on its
 * way (ticket 31). Longer than a normal document takes, so it never flashes up for one, and short
 * enough that a large one never looks like a window that froze or never opened.
 */
export const WAIT_NOTE_MS = 500;

/** The note that says the window's document is on its way, over whatever the view shows. */
export interface WaitNote {
  /** The note, hidden until it shows, for the view to place over the document. */
  readonly element: HTMLElement;
  /** Resolves the first time the note shows, so a window whose document is slow shows with it. */
  readonly shown: Promise<void>;
  /**
   * Waits for a request for the window's document, sent while the view showed `showing`. If
   * `WAIT_NOTE_MS` pass before the returned function is called, as it is when the answer comes,
   * the note shows what the app names for it, while the request is still the latest (`isLatest`)
   * and the note hasn't been hidden since. The app names nothing for a reload, which keeps the
   * document in view untouched instead, and nothing while the window shows no document.
   */
  during(showing: number | null, isLatest: () => boolean): () => void;
  /**
   * The document has arrived and the page is about to take it in, which holds the page for
   * seconds for a large one (ticket 38): the note says so, and its Stop goes, since the page
   * can't answer it then. Returns whether the note shows, so the view paints it first.
   */
  showLayingOut(): boolean;
  /** Whether the reader can stop the document now: the note shows, and it isn't laid out yet. */
  canStop(): boolean;
  /** Hides the note, once the latest request has ended or the reader stopped it. */
  hide(): void;
}

/** "Showing **a.md** (100 MB) ...", the size left out when the app can't tell it. */
function fill(
  text: HTMLElement,
  { file, size }: NonNullable<AnswerOf<"wait_note">>,
  seconds: HTMLElement,
): void {
  const name = document.createElement("strong");
  name.textContent = file.name;
  text.replaceChildren(
    "Showing ",
    name,
    size === null ? " ..." : ` (${size}) ...`,
    seconds,
  );
}

/** "Laying out **a.md** ...". */
function fillLayingOut(text: HTMLElement, fileName: string): void {
  const name = document.createElement("strong");
  name.textContent = fileName;
  text.replaceChildren("Laying out ", name, " ...");
}

/**
 * The note's Stop, named as the title bar's buttons are: by what it does, with its key in the
 * tooltip.
 */
function createStopButton(onStop: () => void): HTMLButtonElement {
  const button = iconButton("Stop", "Stop (Esc)", "Escape", stopGlyph);
  button.addEventListener("click", onStop);
  return button;
}

type WaitNoteListener = (shows: boolean) => void;

const listeners = new Set<WaitNoteListener>();

/**
 * Runs `listener` each time the wait note shows or hides, with whether it shows: while it does,
 * the window waits for a document it doesn't show yet (ticket 31), and the overview ruler hides
 * rather than mark the one it is leaving. Returns a function that removes the listener.
 */
export function whenWaitNoteChanges(listener: WaitNoteListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function tellListeners(shows: boolean): void {
  for (const listener of listeners) listener(shows);
}

/**
 * The wait note, whose Stop (an "x" at its right end) calls `onStop` (ticket 38). It says how
 * many whole seconds the document has been on its way, counted from the request, from the first
 * on, and a later request for the same file, as after a save, goes on counting. The
 * `whenWaitNoteChanges` listeners hear each time it shows or hides.
 */
export function createWaitNote(onStop: () => void): WaitNote {
  const element = document.createElement("div");
  element.className = "wait-note markdown-body";
  element.setAttribute("role", "status");
  element.hidden = true;
  const text = document.createElement("span");
  // Left out of what a screen reader is told of the note, which would otherwise be told again
  // every second.
  const seconds = document.createElement("span");
  seconds.setAttribute("aria-hidden", "true");
  const stop = createStopButton(onStop);
  element.append(text, stop);
  /**
   * Whether a request is on its way since the note was last hidden, so the app's answer for a
   * request that ended meanwhile shows nothing. Not cleared when one request's answer comes: the
   * note stays while the page takes a large document in, until the document replaces it.
   */
  let waiting = false;
  /** The file the note names, and since when its seconds count, while it shows. */
  let named: { name: string; path: string; since: number } | null = null;
  let layingOut = false;
  let tick: ReturnType<typeof setTimeout> | undefined;
  let firstShown: () => void = () => undefined;
  const shown = new Promise<void>((resolve) => {
    firstShown = resolve;
  });

  const stopCounting = () => {
    clearTimeout(tick);
    tick = undefined;
  };

  /** Shows the whole seconds since the count started, and again at the next whole second. */
  const count = () => {
    stopCounting();
    if (named === null) return;
    const elapsed = Date.now() - named.since;
    const whole = Math.floor(elapsed / 1000);
    seconds.textContent = whole >= 1 ? ` ${String(whole)} s` : "";
    tick = setTimeout(count, 1000 - (elapsed % 1000));
  };

  const show = async (
    showing: number | null,
    isLatest: () => boolean,
    requested: number,
  ) => {
    const note = await askApp("wait_note", { showing }).catch(() => null);
    if (note === null || !isLatest() || !waiting) return;
    // The same file again, as after a save during its load, has been on its way all along.
    const since = named?.path === note.file.path ? named.since : requested;
    named = { name: note.file.name, path: note.file.path, since };
    layingOut = false;
    fill(text, note, seconds);
    element.append(stop);
    const wasHidden = element.hidden;
    element.hidden = false;
    count();
    firstShown();
    if (wasHidden) tellListeners(true);
  };

  return {
    element,
    shown,
    during(showing, isLatest) {
      waiting = true;
      const requested = Date.now();
      const timer = setTimeout(() => {
        void show(showing, isLatest, requested);
      }, WAIT_NOTE_MS);
      return () => {
        clearTimeout(timer);
      };
    },
    showLayingOut() {
      if (element.hidden || named === null) return false;
      layingOut = true;
      stopCounting();
      fillLayingOut(text, named.name);
      stop.remove();
      return true;
    },
    canStop() {
      return !element.hidden && !layingOut;
    },
    hide() {
      waiting = false;
      named = null;
      layingOut = false;
      stopCounting();
      const wasShown = !element.hidden;
      element.hidden = true;
      if (wasShown) tellListeners(false);
    },
  };
}
