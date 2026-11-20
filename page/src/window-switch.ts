import { askApp } from "./app";
import { showingWhy } from "./notice";

/**
 * The app's commands that switch a state of the asking window, such as Keep on top and Full
 * screen, and answer whether it is on now.
 */
type SwitchCommand = "switch_keep_on_top" | "switch_full_screen";

type SwitchListener = (on: boolean) => void;

/**
 * A state of the window that the app switches on the window that asks, rather than a setting: a
 * new window, the page with it, starts with it off, and the page shows only what the app last
 * answered (window design ticket 07, full screen ticket 01).
 */
export interface WindowSwitch {
  /**
   * Starts the page as a new window's: off. The signal starts it so again, for the tests, so that
   * no test inherits what another switched on (review fixes of window design ticket 07).
   */
  start(signal?: AbortSignal): void;
  /**
   * Asks the app to switch the state for this window, and resolves once the page knows what the
   * app answered. Switched in the app rather than set to what the page shows, so the answer is the
   * window's state whatever the page thought. Never rejects: a switch that failed says why and
   * leaves the state as it was, as does one answered after the state started afresh.
   */
  switch(): Promise<void>;
  /** Whether the app last answered that the state is on. */
  isOn(): boolean;
  /**
   * Runs `listener` with whether the state is on each time the app's answer changes it. It hears
   * nothing at the start, where every part of a new window's page starts with the state off.
   * Returns a function that removes the listener.
   */
  whenChanges(listener: SwitchListener): () => void;
}

/**
 * The window's state that `command` switches. `whenSwitchedOn` runs on each answer that it is on:
 * the app switches rather than sets, so each is one that switched it on.
 */
export function windowSwitch(
  command: SwitchCommand,
  { whenSwitchedOn }: { whenSwitchedOn?: () => void } = {},
): WindowSwitch {
  let on = false;
  // Counts the times the state started afresh, so that a switch the app answers after that
  // changes nothing: its answer was for the state before.
  let starts = 0;
  const listeners = new Set<SwitchListener>();
  const startAfresh = () => {
    starts++;
    on = false;
  };
  return {
    start(signal) {
      startAfresh();
      signal?.addEventListener("abort", startAfresh);
    },
    async switch() {
      const startAsked = starts;
      const switched = askApp(command);
      showingWhy(switched);
      const answered = await switched.catch(() => undefined);
      if (answered === undefined || startAsked !== starts) return;
      const changed = answered !== on;
      on = answered;
      if (on) whenSwitchedOn?.();
      if (changed) for (const listener of listeners) listener(on);
    },
    isOn: () => on,
    whenChanges(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
