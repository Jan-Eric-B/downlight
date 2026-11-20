import { windowSwitch } from "./window-switch";

/**
 * Keep on top (window design ticket 07): the window stays above all others. It belongs to the
 * window, not the settings (`windowSwitch`).
 */
const keepOnTop = windowSwitch("switch_keep_on_top");

/** Starts the page as a new window's: not kept on top (`WindowSwitch.start`). */
export function startKeepOnTop(signal?: AbortSignal): void {
  keepOnTop.start(signal);
}

/**
 * Asks the app to switch Keep on top for this window, and resolves once the page knows what the
 * app answered, so its button and menu entry can show it. Never rejects (`WindowSwitch.switch`).
 */
export function switchKeepOnTop(): Promise<void> {
  return keepOnTop.switch();
}

/** Whether this window is kept on top, for Keep on top's button and menu entry. */
export function isKeptOnTop(): boolean {
  return keepOnTop.isOn();
}
