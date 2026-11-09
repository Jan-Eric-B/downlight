import "github-markdown-css/github-markdown.css";
import "./document.css";
import { applyBackdrop } from "./backdrop";
import { startCommands } from "./commands";
import { epochMs, watchLongTasks } from "./timing";
import { startDocumentView } from "./view";

const scriptStart = epochMs();
applyBackdrop();
watchLongTasks();
startCommands();
await startDocumentView(scriptStart);
