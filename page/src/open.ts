import { invoke } from "@tauri-apps/api/core";
import { showNotice } from "./notice";

let opening = false;

/** Open: the open dialog, then the chosen document in this window. */
export async function open(): Promise<void> {
  // A second Ctrl+O before the dialog is up would open a second dialog.
  if (opening) return;
  opening = true;
  try {
    await invoke("open_document");
  } catch (error) {
    showNotice(String(error));
  } finally {
    opening = false;
  }
}
