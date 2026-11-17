import { askApp, type ZoomChange } from "./app";
import { showingWhy } from "./notice";
import { whenSettingsChange } from "./settings";
import { keepReadingPosition } from "./view";

/**
 * Zoom (ticket 16): one zoom level that every window shares, kept in the settings. The page zooms
 * only the document view, with CSS `zoom`, so the title bar, the find bar and notices keep their
 * size (window design spec); WebView2's own zoom stays off (`zoom_hotkeys_enabled` in Tauri).
 */

/** The CSS variable the document view's `zoom` reads (document.css), as a factor: 1.25 for 125%. */
export const ZOOM_VARIABLE = "--document-zoom";

/**
 * Asks the app to zoom every window in, out or back to 100%. The app sends every page the new
 * level, this one too, which then zooms (`startZoom`); a level that couldn't be saved says why.
 */
export function changeZoom(change: ZoomChange): void {
  showingWhy(askApp("change_zoom", { change }));
}

/**
 * Zooms the document view to the level in the settings, now and whenever it changes, keeping the
 * reading position. Returns a function that stops it, for the tests.
 */
export function startZoom(): () => void {
  let shown: number | undefined;
  return whenSettingsChange(({ zoom }) => {
    if (zoom === shown) return;
    shown = zoom;
    void keepReadingPosition(() => {
      document.documentElement.style.setProperty(
        ZOOM_VARIABLE,
        String(zoom / 100),
      );
    });
  });
}
