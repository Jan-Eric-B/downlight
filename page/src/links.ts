import { invoke } from "@tauri-apps/api/core";
import { showNotice } from "./notice";
import { DOCUMENT_VIEW_SELECTOR, scrollToHeading } from "./view";

/** What the app's `follow_link` leaves for the page to do (`Followed` in `app/src/links.rs`). */
type Followed =
  { kind: "scrollTo"; heading: string | null } | { kind: "nothing" };

/** The document's link under `target`, the element an event happened on, if there is one. */
function linkAt(target: EventTarget | null): HTMLAnchorElement | null {
  if (!(target instanceof Element)) return null;
  return target.closest<HTMLAnchorElement>(`${DOCUMENT_VIEW_SELECTOR} a[href]`);
}

/**
 * Follows a link through the app, which resolves it with the core's link resolution (ticket 11),
 * and does what is left for the page. The target goes as the document wrote it.
 */
async function follow(target: string, newWindow: boolean): Promise<void> {
  try {
    const followed = await invoke<Followed>("follow_link", {
      target,
      newWindow,
    });
    if (followed.kind === "scrollTo") await scrollToHeading(followed.heading);
  } catch (error) {
    showNotice(String(error));
  }
}

/** The schemes whose link targets the bubble shows as the URL that is opened. */
const NORMALIZED_SCHEMES = new Set(["http:", "https:", "mailto:"]);

/**
 * A link target as the bubble shows it. A web or mail link shows as the URL the system is handed,
 * with its host in punycode and its control characters percent-encoded, so a look-alike letter or
 * a bidi control can't make it pass for another. Paths, relative or not, show as written, since
 * they name files the way the document does.
 */
function shownTarget(target: string): string {
  const url = URL.parse(target);
  return url && NORMALIZED_SCHEMES.has(url.protocol) ? url.href : target;
}

/** The bubble at the bottom left that shows a link's target, as a browser's status bar does. */
function createTargetBubble(): HTMLElement {
  const bubble = document.createElement("div");
  // A markdown-body, for github-markdown-css's font and its light or dark colors.
  bubble.className = "link-target markdown-body";
  // A screen reader reads the link's target from the link itself.
  bubble.setAttribute("aria-hidden", "true");
  bubble.hidden = true;
  document.body.append(bubble);
  return bubble;
}

/**
 * Follows the links in the shown document when they are clicked, and shows a link's target while
 * the pointer is over it or it has the keyboard's focus. The webview never follows a link itself:
 * a click goes to the app, and every other way to open one (Shift+click, the middle button) is
 * stopped. Hovering asks the app nothing, so a link to a network share connects only when
 * clicked. The signal removes the listeners again, for the tests.
 */
export function startLinks(signal?: AbortSignal): void {
  const bubble = createTargetBubble();

  addEventListener(
    "click",
    (event) => {
      const link = linkAt(event.target);
      if (!link) return;
      event.preventDefault();
      if (event.button !== 0 || event.shiftKey || event.altKey || event.metaKey)
        return;
      void follow(link.getAttribute("href") ?? "", event.ctrlKey);
    },
    { signal },
  );
  addEventListener(
    "auxclick",
    (event) => {
      if (linkAt(event.target)) event.preventDefault();
    },
    { signal },
  );

  const showTarget = (event: Event) => {
    const link = linkAt(event.target);
    if (!link) return;
    bubble.textContent = shownTarget(link.getAttribute("href") ?? "");
    bubble.hidden = false;
  };
  /** Hides the target once the pointer or the focus leaves the link, not a part of it. */
  const hideTarget = (event: MouseEvent | FocusEvent) => {
    const link = linkAt(event.target);
    if (!link) return;
    if (
      event.relatedTarget instanceof Node &&
      link.contains(event.relatedTarget)
    )
      return;
    bubble.hidden = true;
  };
  addEventListener("mouseover", showTarget, { signal });
  addEventListener("focusin", showTarget, { signal });
  addEventListener("mouseout", hideTarget, { signal });
  addEventListener("focusout", hideTarget, { signal });
}
