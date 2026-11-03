import { invoke } from "@tauri-apps/api/core";
import "github-markdown-css/github-markdown.css";
import "./document.css";
import { epochMs, reportFirstPaint, watchLongTasks } from "./timing";

const scriptStart = epochMs();
watchLongTasks();

const view = document.createElement("article");
view.className = "markdown-body";
document.body.append(view);

const documentRequested = epochMs();
try {
  // The HTML comes only from the core's render, never from the page.
  const html = await invoke<string>("render_document");
  const documentReceived = epochMs();
  view.innerHTML = html;
  reportFirstPaint({
    scriptStart,
    documentRequested,
    documentReceived,
    documentInserted: epochMs(),
  });
} catch (error) {
  view.textContent = String(error);
}
