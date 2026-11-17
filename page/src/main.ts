import "github-markdown-css/github-markdown.css";
import "./accent.css";
import "./document.css";
import "./title-bar.css";
import { applyBackdrop } from "./backdrop";
import { startCommands } from "./commands";
import { startFind } from "./find";
import { startLinks } from "./links";
import { startSettings } from "./settings";
import { epochMs, watchLongTasks } from "./timing";
import { startTitleBar } from "./title-bar";
import { startDocumentView } from "./view";
import { startZoom } from "./zoom";

const scriptStart = epochMs();
// First, so every part of the page starts with the settings in the page's URL.
const hearingSettings = startSettings();
applyBackdrop();
watchLongTasks();
startTitleBar();
startCommands();
startLinks();
startFind();
// Before the document view starts, so the first paint is at the zoom level already.
startZoom();
// The document view asks for its document only once the page hears the settings: the app holds a
// change made before the first request and sends it on that request (ticket 16's review).
await hearingSettings;
await startDocumentView(scriptStart);
