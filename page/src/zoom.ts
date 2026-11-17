import type { ZoomChange } from "./app";
import { changeSetting } from "./settings";

/**
 * Zoom (ticket 16): one zoom level that every window shares, kept in the settings. The page zooms
 * only the document view, with CSS `zoom`, so the title bar, the find bar and notices keep their
 * size (window design spec); WebView2's own zoom stays off (`zoom_hotkeys_enabled` in Tauri).
 */

/** The CSS variable the document view's `zoom` reads (document.css), as a factor: 1.25 for 125%. */
export const ZOOM_VARIABLE = "--document-zoom";

/**
 * Asks the app to zoom every window in, out or back to 100%. The app sends every page the new
 * level, this one too, which then zooms (`startLayout`); a level that couldn't be saved says why.
 */
export function changeZoom(change: ZoomChange): void {
  changeSetting({ setting: "zoom", change });
}
