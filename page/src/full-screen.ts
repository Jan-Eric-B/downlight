import { showNotice } from "./notice";
import { windowSwitch } from "./window-switch";

/**
 * Full screen (full screen ticket 01): the window fills its whole screen, over the taskbar. It
 * belongs to the window, as Keep on top does, not the settings (`windowSwitch`). Each answer that
 * the window is full screen now says how to leave, so that a reader who pressed F11 by accident
 * knows the way back (full screen spec, story 40); leaving is quiet.
 */
const fullScreen = windowSwitch("switch_full_screen", {
  whenSwitchedOn: () => {
    showNotice("Press F11 or Esc to leave full screen");
  },
});

/** Starts the page as a new window's: out of full screen (`WindowSwitch.start`). */
export function startFullScreen(signal?: AbortSignal): void {
  fullScreen.start(signal);
}

/**
 * Runs `listener` with whether the window is full screen each time the app's answer changes it,
 * for what the page lays out by it rather than shows on the command: the title bar and the room
 * kept for it (full screen ticket 03). Returns a function that removes the listener
 * (`WindowSwitch.whenChanges`).
 */
export function whenFullScreenChanges(
  listener: (fullScreen: boolean) => void,
): () => void {
  return fullScreen.whenChanges(listener);
}

/**
 * Asks the app to switch full screen for this window, and resolves once the page knows what the
 * app answered, so its menu entry can show it. Never rejects (`WindowSwitch.switch`).
 */
export function switchFullScreen(): Promise<void> {
  return fullScreen.switch();
}

/** Whether this window is full screen, for Full screen's menu entry and the title bar. */
export function isFullScreen(): boolean {
  return fullScreen.isOn();
}
