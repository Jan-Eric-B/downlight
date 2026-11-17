import { firstSettings, startListeningToApp, type PageSettings } from "./app";

/**
 * The settings every window shares, as the page shows them (window design ticket 12): the first
 * snapshot comes in the page's URL, so first paint waits for no round trip, and each change in one
 * `settings` message to every window. A setting the page shows is a field of the snapshot and a
 * listener here, rather than a message, a URL flag and a fake-app answer of its own.
 */

type SettingsListener = (settings: PageSettings) => void;

const listeners = new Set<SettingsListener>();

/** The snapshot the page shows now, once `startSettings` has run. */
let current: PageSettings | undefined;

/**
 * Starts hearing the settings: the snapshot in the page's URL now, and each one the app sends
 * after. Before the page's other parts start, so each of them starts with the settings. Resolves
 * once the page hears the app's `settings` messages: the app holds a change made before the page
 * first asks for its document and sends it then (`Documents::page_hears_settings` in
 * `app/src/documents.rs`), so the page asks only after this (`main.ts`).
 */
export function startSettings(): Promise<void> {
  current = firstSettings();
  return startListeningToApp("settings", (settings) => {
    current = settings;
    for (const listener of listeners) listener(settings);
  });
}

/**
 * Runs `listener` with the settings now, and again with each new snapshot the app sends. Returns
 * a function that removes the listener. Throws before `startSettings`, which would leave the
 * listener deaf to every change.
 */
export function whenSettingsChange(listener: SettingsListener): () => void {
  if (current === undefined) {
    throw new Error("The page hears the settings only after startSettings().");
  }
  listeners.add(listener);
  listener(current);
  return () => {
    listeners.delete(listener);
  };
}
