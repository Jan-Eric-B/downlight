/** How long a notice stays, long enough to read one sentence. */
const SHOWN_MS = 4000;

let hideTimer: ReturnType<typeof setTimeout> | undefined;

/**
 * Shows a short message at the bottom of the window for a few seconds: why a dropped file
 * didn't open, or why a document couldn't be shown.
 */
export function showNotice(message: string): void {
  let notice = document.querySelector<HTMLElement>(".notice");
  if (!notice) {
    notice = document.createElement("div");
    // A markdown-body, for github-markdown-css's font and its light or dark colors.
    notice.className = "notice markdown-body";
    notice.setAttribute("role", "status");
    document.body.append(notice);
  }
  const shown = notice;
  shown.textContent = message;
  shown.hidden = false;
  clearTimeout(hideTimer);
  hideTimer = setTimeout(() => {
    shown.hidden = true;
  }, SHOWN_MS);
}
