import { askApp } from "./app";
import { showingWhy } from "./notice";

/**
 * Keep on top (window design ticket 07): the window stays above all others. It belongs to the
 * window, not the settings: the app switches it on the window that asks, and a new window, the
 * page with it, starts without it. The page shows only what the app last answered.
 */

/** Whether the app last answered that this window is kept on top. */
let keptOnTop = false;

/**
 * Counts the times the state started afresh, so that a switch the app answers after that changes
 * nothing: its answer was for the state before.
 */
let starts = 0;

/** Starts the state afresh, as a new window's page: not kept on top. */
function startAfresh(): void {
  starts++;
  keptOnTop = false;
}

/**
 * Starts the page as a new window's: not kept on top. The signal starts it so again, for the
 * tests, so that no test inherits what another switched on (review fixes of window design ticket
 * 07).
 */
export function startKeepOnTop(signal?: AbortSignal): void {
  startAfresh();
  signal?.addEventListener("abort", startAfresh);
}

/**
 * Asks the app to switch Keep on top for this window, and resolves once the page knows what the
 * app answered, so its button and menu entry can show it. Switched in the app rather than set to
 * what the page shows, so the answer is the window's state whatever the page thought. Never
 * rejects: a switch that failed says why and leaves the state as it was, as does one answered
 * after the state started afresh.
 */
export async function switchKeepOnTop(): Promise<void> {
  const startAsked = starts;
  const switched = askApp("switch_keep_on_top");
  showingWhy(switched);
  const answered = await switched.catch(() => keptOnTop);
  if (startAsked === starts) keptOnTop = answered;
}

/** Whether this window is kept on top, for Keep on top's button and menu entry. */
export function isKeptOnTop(): boolean {
  return keptOnTop;
}
