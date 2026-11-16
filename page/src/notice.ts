import { askApp, type NamedFile, type Notice } from "./app";

/** How long a notice stays after its last message, long enough to read one sentence. */
const SHOWN_MS = 4000;

/** The most messages a notice shows at once; another one drops the oldest. */
const MOST_MESSAGES = 3;

/** A folder that opens from a notice. */
const FOLDER_LINK = ".folder-link";

let hideTimer: ReturnType<typeof setTimeout> | undefined;

/**
 * Whether the pointer is on the notice, and whether the focus is in it, which hold a notice with
 * a link (window design ticket 11): it holds at most that one small action, and stays while the
 * action is in reach.
 */
let pointerOn = false;
let focusOn = false;

function holds(notice: HTMLElement): boolean {
  return (pointerOn || focusOn) && notice.querySelector(FOLDER_LINK) !== null;
}

/** Hides the notice 4 s from now, unless the reader is at its link. */
function hideLater(notice: HTMLElement): void {
  clearTimeout(hideTimer);
  if (holds(notice)) return;
  hideTimer = setTimeout(() => {
    notice.hidden = true;
    notice.replaceChildren();
    // Nothing is in a hidden notice, and the next one may show away from the pointer.
    pointerOn = false;
    focusOn = false;
  }, SHOWN_MS);
}

/**
 * Runs `change` on the pointer or the focus, and holds or lets go of the notice if that changed
 * whether it holds; a notice that holds nothing keeps the time it had.
 */
function onHoldChange(notice: HTMLElement, change: () => void): void {
  const held = holds(notice);
  change();
  if (holds(notice) !== held) hideLater(notice);
}

function createNotice(): HTMLElement {
  const notice = document.createElement("div");
  // A markdown-body, for github-markdown-css's font and its light or dark colors.
  notice.className = "notice markdown-body";
  notice.setAttribute("role", "status");
  pointerOn = false;
  focusOn = false;
  notice.addEventListener("pointerenter", () => {
    onHoldChange(notice, () => (pointerOn = true));
  });
  notice.addEventListener("pointerleave", () => {
    onHoldChange(notice, () => (pointerOn = false));
  });
  notice.addEventListener("focusin", () => {
    onHoldChange(notice, () => (focusOn = true));
  });
  notice.addEventListener("focusout", (event) => {
    const stays =
      event.relatedTarget instanceof Node &&
      notice.contains(event.relatedTarget);
    onHoldChange(notice, () => (focusOn = stays));
  });
  document.body.append(notice);
  return notice;
}

/**
 * Shows a short message at the bottom of the window for a few seconds: why a dropped file didn't
 * open, or why a document couldn't be shown. A message that comes while others still show goes on
 * a line below them rather than over them, since two can come at once (a held "Can't watch"
 * notice and a failed drop), and all go 4 s after the last. A notice whose folder opens stays
 * while the pointer is on it or the focus in it, and goes 4 s after.
 */
export function showNotice(message: Notice): void {
  const notice =
    document.querySelector<HTMLElement>(".notice") ?? createNotice();
  const line = document.createElement("div");
  line.append(noticeContent(message));
  notice.append(line);
  while (notice.childElementCount > MOST_MESSAGES) {
    notice.firstElementChild?.remove();
  }
  notice.hidden = false;
  hideLater(notice);
}

/**
 * The text of `notice`, with the file it names as its name in bold and its folder after it, which
 * opens on a click when the file is missing and its folder there (window design ticket 11). The
 * one place that builds a notice, for the notices and for the view's "can't read" alike.
 */
export function noticeContent(notice: Notice): DocumentFragment {
  const content = document.createDocumentFragment();
  if (typeof notice === "string") content.append(notice);
  else content.append(notice.before, ...namedFile(notice.file), notice.after);
  return content;
}

/** The name in bold, and " in " and the folder after it if it has one. */
function namedFile(file: NamedFile): Node[] {
  const name = document.createElement("strong");
  name.className = "unbreakable";
  name.textContent = file.name;
  if (file.folder === null) return [name];
  return [
    name,
    document.createTextNode(" in "),
    folderOf(file.folder, file.folderOpens),
  ];
}

/**
 * The folder, which breaks only after a separator, never inside a name (a name breaks after its
 * hyphens otherwise), and which opens on a click or Enter if it `opens`.
 */
function folderOf(folder: string, opens: boolean): HTMLElement {
  const element = document.createElement("span");
  const parts = (folder.match(/[^\\/]*(?:[\\/]+|$)/g) ?? []).filter(
    (part) => part !== "",
  );
  parts.forEach((part, index) => {
    if (index > 0) element.append(document.createElement("wbr"));
    const unbreakable = document.createElement("span");
    unbreakable.className = "unbreakable";
    unbreakable.textContent = part;
    element.append(unbreakable);
  });
  if (!opens) return element;
  // A link, not a button, so it wraps with the text: a button is laid out as one box.
  element.className = FOLDER_LINK.slice(1);
  element.setAttribute("role", "link");
  element.tabIndex = 0;
  element.setAttribute("aria-label", `Open the folder ${folder}`);
  const open = () => {
    // The folder as the app named it: the app opens no other.
    showingWhy(askApp("open_named_folder", { folder }));
  };
  element.addEventListener("click", open);
  element.addEventListener("keydown", (event) => {
    if (event.key === "Enter") open();
  });
  // A click leaves the focus where it is, so the notice doesn't stay after the folder opened
  // until the reader clicks elsewhere; Tab still reaches the link.
  element.addEventListener("mousedown", (event) => {
    event.preventDefault();
  });
  return element;
}

/**
 * Why a command failed, as the reader is told it: the app's notice, or what Tauri gave when the
 * app couldn't be asked.
 */
function whyItFailed(error: unknown): Notice {
  if (typeof error === "string") return error;
  if (typeof error === "object" && error !== null && "file" in error)
    return error as Notice;
  return String(error);
}

/** Shows the reader why, in a notice, if the app couldn't do what it was `asked`. */
export function showingWhy(asked: Promise<unknown>): void {
  asked.catch((error: unknown) => {
    showNotice(whyItFailed(error));
  });
}
