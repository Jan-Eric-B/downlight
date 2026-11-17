import { changeSetting, settingsNow } from "./settings";

/**
 * Full width (window design ticket 06): the document fills the window instead of GitHub's reading
 * width. One choice for every window, kept in the settings, so it reaches the page as a field of
 * the settings snapshot (ticket 12) and every new window starts with it.
 */

/** The class on the page's root while the document fills the window's width (document.css). */
export const FULL_WIDTH_CLASS = "full-width";

/**
 * Asks the app to switch Full width for every window. The app sends every page the settings,
 * this one too, which then widens or narrows (`startLayout`); a choice that couldn't be saved
 * says why.
 */
export function switchFullWidth(): void {
  changeSetting({ setting: "switchFullWidth" });
}

/** Whether the document fills the window's width, for Full width's button and menu entry. */
export function isFullWidth(): boolean {
  return settingsNow().fullWidth;
}
