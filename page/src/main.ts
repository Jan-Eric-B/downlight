import { invoke } from "@tauri-apps/api/core";
import "github-markdown-css/github-markdown.css";
import "./document.css";

const view = document.createElement("article");
view.className = "markdown-body";
document.body.append(view);

try {
  // The HTML comes only from the core's render, never from the page.
  view.innerHTML = await invoke<string>("render_document");
} catch (error) {
  view.textContent = String(error);
}
