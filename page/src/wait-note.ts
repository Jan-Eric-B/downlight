import { askApp, type AnswerOf } from "./app";

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
  /** Hides the note, once the latest request has ended. */
  hide(): void;
}

/** "Showing **a.md** (100 MB) ...", the size left out when the app can't tell it. */
function fill(
  note: HTMLElement,
  { file, size }: NonNullable<AnswerOf<"wait_note">>,
): void {
  const name = document.createElement("strong");
  name.textContent = file.name;
  note.replaceChildren(
    "Showing ",
    name,
    size === null ? " ..." : ` (${size}) ...`,
  );
}

export function createWaitNote(): WaitNote {
  const element = document.createElement("div");
  element.className = "wait-note markdown-body";
  element.setAttribute("role", "status");
  element.hidden = true;
  /**
   * Whether a request is on its way since the note was last hidden, so the app's answer for a
   * request that ended meanwhile shows nothing. Not cleared when one request's answer comes: the
   * note stays while the page takes a large document in, until the document replaces it.
   */
  let waiting = false;
  let firstShown: () => void = () => undefined;
  const shown = new Promise<void>((resolve) => {
    firstShown = resolve;
  });

  const show = async (showing: number | null, isLatest: () => boolean) => {
    const named = await askApp("wait_note", { showing }).catch(() => null);
    if (named === null || !isLatest() || !waiting) return;
    fill(element, named);
    element.hidden = false;
    firstShown();
  };

  return {
    element,
    shown,
    during(showing, isLatest) {
      waiting = true;
      const timer = setTimeout(() => {
        void show(showing, isLatest);
      }, WAIT_NOTE_MS);
      return () => {
        clearTimeout(timer);
      };
    },
    hide() {
      waiting = false;
      element.hidden = true;
    },
  };
}
