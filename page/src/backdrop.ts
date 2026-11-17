import { pageFlags } from "./app";

/**
 * Marks the page as sitting on the system's backdrop (Mica on Windows) when the app says so, so
 * its stylesheet draws no background of its own. Without it, as on Windows 10, the page draws a
 * solid background.
 */
export function applyBackdrop(): void {
  document.documentElement.classList.toggle(
    "backdrop",
    pageFlags().backdropSupported,
  );
}
