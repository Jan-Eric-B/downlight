/**
 * A small bubble in a bottom corner of the window, over the document, as the link target bubble
 * (links.ts) and the selection's counts (counts.ts) show: `name` is its class, which puts it in
 * its corner (`.corner-bubble` in `document.css`). Hidden until its text is set, and from screen
 * readers always, which read what it shows from the document itself.
 */
export function createCornerBubble(name: string): HTMLElement {
  const bubble = document.createElement("div");
  // A markdown-body, for github-markdown-css's font and its light or dark colors.
  bubble.className = `corner-bubble ${name} markdown-body`;
  bubble.setAttribute("aria-hidden", "true");
  bubble.hidden = true;
  document.body.append(bubble);
  return bubble;
}
