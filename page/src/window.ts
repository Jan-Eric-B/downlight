import { invoke } from "@tauri-apps/api/core";

/**
 * Ctrl+W closes the window. On a layout that types Latin letters it goes by the letter the key
 * types, so it follows AZERTY and the like; on other layouts, such as Cyrillic, by the key's
 * place, as other Windows apps do.
 */
export function closeOnCtrlW(): void {
  addEventListener("keydown", (event) => {
    if (!event.ctrlKey || event.altKey || event.shiftKey || event.metaKey) {
      return;
    }
    const isLatinLetter = /^[a-z]$/i.test(event.key);
    const isW = isLatinLetter
      ? event.key.toLowerCase() === "w"
      : event.code === "KeyW";
    if (!isW) return;
    event.preventDefault();
    // Closing the window can't fail in a way the reader could act on: the window stays open.
    void invoke("close_window");
  });
}
