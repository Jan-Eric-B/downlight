/**
 * The page's part of the window's history (ticket 12). The app keeps which documents the window
 * showed before and after the one it shows (`app/src/documents/history.rs`), each by the showing
 * it had; the page keeps, for each showing, where its view was in it: one visit when the window
 * showed it, and one more for each heading a link scrolled to in it, each with the reading
 * position the view had when it last left that visit, and the showing's ticks. Back and Forward step through a showing's
 * visits here first, and ask the app for another document only past its first or last one, so a
 * step within a document never reads its file again. The history is never saved, and goes with
 * the page when its window closes.
 */
import type { Step, Steps } from "./app";
import { NO_TICKS, type Ticks } from "./ticks";

/** A reading position, as a line number, or undefined at the start of the document. */
export type Line = number | undefined;

/**
 * What the page keeps of a showing: its visits, by the reading position each was left at, the
 * one the view is at, and its ticks (ticket 13). Ticks belong to the showing rather than to a
 * visit, since a heading link within a document changes none, so they are read as the view
 * leaves the showing and given back on a return.
 */
interface Kept {
  lines: Line[];
  at: number;
  ticks: Ticks;
}

/** What a showing the window shows anew starts with: the one visit, and no ticks. */
function keptAnew(): Kept {
  return { lines: [undefined], at: 0, ticks: NO_TICKS };
}

/** The visits of the showings the window shows or has shown, and whether the app can step. */
export class WindowHistory {
  private readonly kept = new Map<number, Kept>();

  /** Whether the app has a document to go back or forward to, as its last answer said. */
  private documents: Steps = { back: false, forward: false };

  /** The app's word, with its latest answer, on whether it has a document to go back or forward to. */
  appCanStep(steps: Steps): void {
    this.documents = steps;
  }

  /**
   * The view leaves `showing`, the view's top at `line`, with `ticks`: the visit it is at keeps
   * the line, and the showing the ticks, for a step back to it.
   */
  leave(showing: number, line: Line, ticks: Ticks): void {
    const kept = this.keptOf(showing);
    kept.lines[kept.at] = line;
    kept.ticks = ticks;
  }

  /**
   * The window shows `showing` anew, after `left`, the showing the view showed before, or null:
   * `showing` has the one visit, and `left` loses the visits after the one it was left at, as the
   * app's history loses the documents after it.
   */
  showAnew(showing: number | null, left: number | null): void {
    if (left !== null) this.dropAfter(this.keptOf(left));
    if (showing !== null) this.kept.set(showing, keptAnew());
  }

  /** Where a return to `showing` lands: the reading position its current visit was left at. */
  returnTo(showing: number): Line {
    const kept = this.keptOf(showing);
    return kept.lines[kept.at];
  }

  /** The ticks `showing` had when the view last left it, for a return to it. */
  ticksOf(showing: number): Ticks {
    return this.keptOf(showing).ticks;
  }

  /**
   * A link scrolls the view, which shows `showing` with its top at `line`, to a heading: one more
   * visit after the current one, in place of any after it, and no document after it either.
   * Returns whether the app had one, which it is then to forget (`forget_forward`).
   */
  visitHeading(showing: number, line: Line): boolean {
    const kept = this.keptOf(showing);
    kept.lines[kept.at] = line;
    this.dropAfter(kept);
    kept.lines.push(undefined);
    kept.at += 1;
    const hadForward = this.documents.forward;
    this.documents = { ...this.documents, forward: false };
    return hadForward;
  }

  /**
   * Takes `step` within `showing`, whose view has its top at `line`, and gives the reading
   * position of the visit it goes to, or undefined if there is none that way in this showing.
   */
  stepWithin(
    showing: number,
    step: Step,
    line: Line,
  ): { line: Line } | undefined {
    const kept = this.keptOf(showing);
    const to = kept.at + (step === "back" ? -1 : 1);
    if (to < 0 || to >= kept.lines.length) return undefined;
    kept.lines[kept.at] = line;
    kept.at = to;
    return { line: kept.lines[to] };
  }

  /** Whether `step` goes anywhere from `showing`, the showing in view, or from no showing. */
  canStep(showing: number | null, step: Step): boolean {
    if (this.documents[step]) return true;
    if (showing === null) return false;
    const { lines, at } = this.keptOf(showing);
    return step === "back" ? at > 0 : at < lines.length - 1;
  }

  /**
   * What the page keeps of `showing`. One it has never seen, as after the app answered a request
   * the page overtook, starts with the one visit.
   */
  private keptOf(showing: number): Kept {
    let kept = this.kept.get(showing);
    if (!kept) {
      kept = keptAnew();
      this.kept.set(showing, kept);
    }
    return kept;
  }

  private dropAfter(kept: Kept): void {
    kept.lines.length = kept.at + 1;
  }
}
