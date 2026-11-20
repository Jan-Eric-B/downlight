/**
 * One of the page's icon buttons, showing `glyph`, in the title bar's look (`.icon-button` in
 * `document.css`): the find bar's previous, next and close (find.ts) and the wait note's Stop
 * (wait-note.ts). Named `label` for screen readers, with `tooltip` naming the key that does the
 * same (`keys`), as the title bar's buttons name their shortcuts. A press keeps the focus where it
 * was, in Find's box while the reader types, so Enter and typing go on working after a click.
 */
export function iconButton(
  label: string,
  tooltip: string,
  keys: string,
  glyph: string,
): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "icon-button";
  button.setAttribute("aria-label", label);
  button.setAttribute("aria-keyshortcuts", keys);
  button.title = tooltip;
  // The SVG is one of the page's own bundled glyphs.
  button.innerHTML = glyph;
  button.addEventListener("mousedown", (event) => {
    event.preventDefault();
  });
  return button;
}
