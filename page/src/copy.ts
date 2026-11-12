import { showNotice } from "./notice";

/** How long the button shows that it copied. */
const COPIED_MS = 2000;

/**
 * Gives each code block in the shown document a button that copies its code (ticket 07).
 *
 * The block is wrapped as on GitHub, `div.highlight > pre + div.zeroclipboard-container`, which
 * github-markdown-css spaces for the button. The button sits beside the `pre` rather than in it,
 * since a `pre` scrolls sideways and would take the button with it. It has no text, only an icon
 * and a label for screen readers, so Find and a copied selection never pick up its name.
 */
export function addCopyButtons(view: HTMLElement): void {
  for (const pre of view.querySelectorAll("pre")) {
    const code = pre.querySelector("code");
    if (!code) continue;
    const container = document.createElement("div");
    container.className = "zeroclipboard-container";
    container.append(copyButton(code));
    const block = document.createElement("div");
    block.className = "highlight";
    pre.replaceWith(block);
    block.append(pre, container);
  }
}

function copyButton(code: HTMLElement): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "copy-code";
  showCopied(button, false);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  button.addEventListener("click", () => {
    void navigator.clipboard.writeText(code.textContent).then(
      () => {
        showCopied(button, true);
        // A copy made while the button still shows the last one starts the 2 s again.
        clearTimeout(copiedTimer);
        copiedTimer = setTimeout(() => {
          showCopied(button, false);
        }, COPIED_MS);
      },
      (error: unknown) => {
        showNotice(`Can't copy the code: ${String(error)}`);
      },
    );
  });
  return button;
}

function showCopied(button: HTMLButtonElement, copied: boolean): void {
  const name = copied ? "Copied" : "Copy code";
  button.classList.toggle("copied", copied);
  button.setAttribute("aria-label", name);
  button.title = name;
}
