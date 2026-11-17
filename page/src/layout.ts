import type { PageSettings } from "./app";
import { FULL_WIDTH_CLASS } from "./full-width";
import { whenSettingsChange } from "./settings";
import { keepReadingPosition } from "./view";
import { ZOOM_VARIABLE } from "./zoom";

/**
 * The settings that lay out the document view: zoom (ticket 16) and Full width (window design
 * ticket 06). Both wrap the text anew, so they change together, in one layout change.
 */
type Layout = Pick<PageSettings, "zoom" | "fullWidth">;

/**
 * Lays the document view out as the settings say, now and whenever they change, keeping the
 * reading position. A snapshot that changes both zoom and Full width wraps the document anew once
 * rather than twice, which counts with a 1 MB document, and the view never lands on a layout with
 * only one of them in place. Find hears nothing: its matches are ranges in the content, which
 * stays as it was. Returns a function that stops it, for the tests.
 */
export function startLayout(): () => void {
  let shown: Layout | undefined;
  return whenSettingsChange(({ zoom, fullWidth }) => {
    if (shown?.zoom === zoom && shown.fullWidth === fullWidth) return;
    shown = { zoom, fullWidth };
    void keepReadingPosition(() => {
      const root = document.documentElement;
      root.style.setProperty(ZOOM_VARIABLE, String(zoom / 100));
      root.classList.toggle(FULL_WIDTH_CLASS, fullWidth);
    });
  });
}
