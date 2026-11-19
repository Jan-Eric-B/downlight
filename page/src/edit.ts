import { askApp } from "./app";
import { showingWhy } from "./notice";

/** Edit: the document opened in the reader's editor, which the app picks (ticket 15). */
export function edit(): void {
  showingWhy(askApp("edit_document"));
}

let choosing = false;

/**
 * Choose... under Edit with in the settings flyout (window design ticket 08): the app's file
 * dialog for a program, kept as the reader's editor. Resolves once the dialog has closed, for the
 * flyout to show the editor chosen; never rejects, as a choice that can't be kept says why.
 */
export async function chooseEditor(): Promise<void> {
  // A second Choose... before the dialog is up would open a second dialog.
  if (choosing) return;
  choosing = true;
  const asked = askApp("choose_editor");
  showingWhy(asked);
  try {
    await asked;
  } catch {
    // Shown by `showingWhy`.
  } finally {
    choosing = false;
  }
}
