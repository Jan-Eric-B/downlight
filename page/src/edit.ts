import { askApp } from "./app";
import { showingWhy } from "./notice";

/** Edit: the document opened in the reader's editor, which the app picks (ticket 15). */
export function edit(): void {
  showingWhy(askApp("edit_document"));
}

let choosing = false;

/** Choose editor...: the app's file dialog for a program, kept as the reader's editor. */
export function chooseEditor(): void {
  // A second Choose editor before the dialog is up would open a second dialog.
  if (choosing) return;
  choosing = true;
  showingWhy(
    askApp("choose_editor").finally(() => {
      choosing = false;
    }),
  );
}
