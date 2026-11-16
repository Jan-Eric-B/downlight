import { askApp } from "./app";
import { showingWhy } from "./notice";

let opening = false;

/** Open: the open dialog, then the chosen document in this window. */
export function open(): void {
  // A second Ctrl+O before the dialog is up would open a second dialog.
  if (opening) return;
  opening = true;
  showingWhy(
    askApp("open_document").finally(() => {
      opening = false;
    }),
  );
}
