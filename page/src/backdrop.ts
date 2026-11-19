import { pageFlags } from "./app";
import { whenSettingChanges } from "./settings";

/**
 * Marks the page as sitting on the window's backdrop (Mica on Windows 11), so its stylesheet draws
 * no background of its own, while the OS can show one (the app's `backdropSupported` flag) and the
 * reader's Backdrop isn't None; it follows the reader's changes from any window (window design
 * ticket 08). Without a backdrop, as on Windows 10 or with Backdrop None, the page draws a solid
 * background. After `startSettings`. Returns a function that stops following, for the tests.
 */
export function startBackdrop(): () => void {
  const { backdropSupported } = pageFlags();
  return whenSettingChanges("backdrop", (backdrop) => {
    document.documentElement.classList.toggle(
      "backdrop",
      backdropSupported && backdrop !== "none",
    );
  });
}
