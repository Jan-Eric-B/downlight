import "github-markdown-css/github-markdown.css";
import "./document.css";
import { applyBackdrop } from "./backdrop";
import { startCommands } from "./commands";
import { startFind } from "./find";
import { startLinks } from "./links";
import { epochMs, watchLongTasks } from "./timing";
import { startDocumentView } from "./view";

const scriptStart = epochMs();
applyBackdrop();
watchLongTasks();
startCommands();
startLinks();
startFind();
await startDocumentView(scriptStart);
