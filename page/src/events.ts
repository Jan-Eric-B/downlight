import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { showNotice } from "./notice";

/**
 * Runs `handler` on each `event` the app sends this window. The app sends each such event to one
 * window, and Tauri delivers it only to listeners that name that window, never to a plain
 * `listen`. Every payload the app sends is a string, or nothing.
 */
export function listenToApp(
  event: string,
  handler: (payload: string) => void,
): void {
  getCurrentWebviewWindow()
    .listen<string>(event, ({ payload }) => {
      handler(payload);
    })
    .catch((error: unknown) => {
      // Without the listener the window would miss what the app tells it.
      showNotice(String(error));
    });
}
