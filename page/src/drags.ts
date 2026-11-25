import { DRAG_TAG, windowLabel } from "./app";

/**
 * Tags each drag that starts in the page, a selection, a link or an image, with its window's
 * label (paste-markdown ticket 04). The app's drop target takes drops for the whole window and
 * refuses a drag tagged with it, so one dropped back where it started does nothing, while one
 * from another Downlight window is taken as any program's is. The page handles no drops itself
 * (ADR 0007). Returns what stops it, for the tests.
 */
export function startTaggingDrags(): () => void {
  const tag = (event: DragEvent): void => {
    event.dataTransfer?.setData(DRAG_TAG, windowLabel());
  };
  // Captured on the document, so no element that handles its own drags can keep one untagged.
  document.addEventListener("dragstart", tag, { capture: true });
  return () => {
    document.removeEventListener("dragstart", tag, { capture: true });
  };
}
