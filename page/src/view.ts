import { invoke } from "@tauri-apps/api/core";
import { open } from "./commands";
import { listenToApp } from "./events";
import { showNotice } from "./notice";
import { epochMs, reportFirstPaint } from "./timing";

/** What a window without a document shows: a way to open one. */
function createEmptyState(): HTMLElement {
  const empty = document.createElement("main");
  empty.className = "empty-window markdown-body";
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = "Open file";
  button.addEventListener("click", () => void open());
  const hint = document.createElement("p");
  hint.textContent = "or drop a file here";
  empty.append(button, hint);
  return empty;
}

/**
 * Shows the window's document, or the empty state while it has none, and shows the next one
 * whenever the app says the window shows another document. `scriptStart` is when the page's
 * script started, for the first paint report.
 */
export async function startDocumentView(scriptStart: number): Promise<void> {
  const view = document.createElement("article");
  view.className = "markdown-body";
  const empty = createEmptyState();
  empty.hidden = true;
  document.body.append(view, empty);

  let latestRequest = 0;

  /**
   * Asks the app for the window's document and shows it. Returns when the HTML arrived, if this
   * request inserted a document; a request that a later one overtook shows nothing, so the
   * window ends up with the document it was shown last.
   */
  const showDocument = async (): Promise<number | undefined> => {
    const request = ++latestRequest;
    try {
      const html = await invoke<string | null>("render_document");
      if (request !== latestRequest) return undefined;
      const received = epochMs();
      view.hidden = html === null;
      empty.hidden = html !== null;
      // The HTML comes only from the core's render, never from the page.
      view.innerHTML = html ?? "";
      return html === null ? undefined : received;
    } catch (error) {
      if (request !== latestRequest) return undefined;
      view.hidden = false;
      empty.hidden = true;
      view.textContent = String(error);
      return undefined;
    }
  };

  // The listeners only start: waiting for them would hold up first paint.
  listenToApp("document-shown", () => {
    void showDocument().then(() => {
      scrollTo(0, 0);
    });
  });
  listenToApp("notice", showNotice);

  const documentRequested = epochMs();
  const documentReceived = await showDocument();
  if (documentReceived !== undefined) {
    reportFirstPaint({
      scriptStart,
      documentRequested,
      documentReceived,
      documentInserted: epochMs(),
    });
  }
}
