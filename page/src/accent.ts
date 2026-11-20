import { ACCENT_SHADES, type AccentScale } from "./app";
import { whenSettingsChange } from "./settings";

/**
 * Colors the page with Windows' accent color while the reader's accent color is Windows' (window
 * design ticket 09), following the reader's choice and Windows' changes from the settings, and
 * with amber, `accent.css`'s own scale, otherwise. After `startSettings`. Returns a function that
 * stops following and shows amber again, for the tests.
 */
export function startAccent(): () => void {
  /** The scale shown now, as JSON, so a snapshot that changes something else sets nothing. */
  let shown: string | null = null;
  const stopFollowing = whenSettingsChange((settings) => {
    const scale =
      settings.accentColor === "windows" ? settings.windowsAccent : null;
    const showing = scale === null ? null : JSON.stringify(scale);
    if (showing === shown) return;
    shown = showing;
    showScale(scale);
  });
  return () => {
    stopFollowing();
    showScale(null);
  };
}

/**
 * Sets `scale`'s shades on the root element, over `accent.css`'s amber, or removes them for amber
 * again. Inline on the root, the element whose rule declares the accent's roles: a custom
 * property inherits as its computed value, so shades set on an element below the root would leave
 * the roles taking amber's (ticket 02's review). By the shade list alone, never by the `--accent-`
 * prefix, which the roles share (the review of tickets 03 and 04).
 */
function showScale(scale: AccentScale | null): void {
  const { style } = document.documentElement;
  for (const shade of ACCENT_SHADES) {
    const variable = `--accent-${shade}`;
    if (scale === null) style.removeProperty(variable);
    else style.setProperty(variable, scale[shade]);
  }
}
