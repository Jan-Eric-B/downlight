import {
  askApp,
  firstSettings,
  startListeningToApp,
  type PageSettings,
  type SettingChange,
} from "./app";
import { showingWhy } from "./notice";

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
 * Asks the app to change a setting for every window. The app sends every page the settings, this
 * one too, which its listeners then show; a change that couldn't be saved says why. A setting the
 * reader changes is a variant of `SettingChange` rather than a command of its own. Resolves once
 * the app has answered, for a change the page shows nothing of in the settings, such as Edit
 * with's Default (window design ticket 08); never rejects.
 */
export function changeSetting(change: SettingChange): Promise<void> {
  const asked = askApp("change_setting", { change });
  showingWhy(asked);
  return asked.then(
    () => undefined,
    () => undefined,
  );
}

/**
 * The settings the page shows now, for a command whose button or menu entry shows one, such as
 * Full width: read when it is drawn, so it never waits for a listener that may run after the one
 * drawing it. Throws before `startSettings`, as `whenSettingsChange` does.
 */
export function settingsNow(): PageSettings {
  return started();
}

/**
 * Runs `listener` with the settings now, and again with each new snapshot the app sends. Returns
 * a function that removes the listener. Throws before `startSettings`, which would leave the
 * listener deaf to every change.
 */
export function whenSettingsChange(listener: SettingsListener): () => void {
  const settings = started();
  listeners.add(listener);
  listener(settings);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * Runs `listener` with the setting `key` now, and again only when a new snapshot changes it, for
 * a setting the page shows that doesn't lay out the document, such as the backdrop (window design
 * ticket 08). Settings that lay out the document view (zoom, Full width) go through `startLayout`
 * instead, which changes them together so the document wraps anew once. Returns a function that
 * removes the listener; throws before `startSettings`, as `whenSettingsChange` does.
 */
export function whenSettingChanges<K extends keyof PageSettings>(
  key: K,
  listener: (value: PageSettings[K]) => void,
): () => void {
  let last: PageSettings[K] | undefined;
  return whenSettingsChange((settings) => {
    const value = settings[key];
    if (value === last) return;
    last = value;
    listener(value);
  });
}

/**
 * The snapshot now, or an error before `startSettings`: a part of the page that read the settings
 * sooner would show the defaults rather than the reader's (review fixes of window design ticket 06).
 */
function started(): PageSettings {
  if (current === undefined) {
    throw new Error("The page hears the settings only after startSettings().");
  }
  return current;
}
