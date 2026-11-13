/** How long a notice stays after its last message, long enough to read one sentence. */
const SHOWN_MS = 4000;

/** The most messages a notice shows at once; another one drops the oldest. */
const MOST_MESSAGES = 3;

let hideTimer: ReturnType<typeof setTimeout> | undefined;

function createNotice(): HTMLElement {
  const notice = document.createElement("div");
  // A markdown-body, for github-markdown-css's font and its light or dark colors.
  notice.className = "notice markdown-body";
  notice.setAttribute("role", "status");
  document.body.append(notice);
  return notice;
}

/**
 * Shows a short message at the bottom of the window for a few seconds: why a dropped file
 * didn't open, or why a document couldn't be shown. A message that comes while others still show
 * goes on a line below them rather than over them, since two can come at once (a held "Can't
 * watch" notice and a failed drop), and all go 4 s after the last.
 */
export function showNotice(message: string): void {
  const notice =
    document.querySelector<HTMLElement>(".notice") ?? createNotice();
  const line = document.createElement("div");
  line.textContent = message;
  notice.append(line);
  while (notice.childElementCount > MOST_MESSAGES) {
    notice.firstElementChild?.remove();
  }
  notice.hidden = false;
  clearTimeout(hideTimer);
  hideTimer = setTimeout(() => {
    notice.hidden = true;
    notice.replaceChildren();
  }, SHOWN_MS);
}
