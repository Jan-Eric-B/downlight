import "github-markdown-css/github-markdown.css";
import "./accent.css";
import "./document.css";
import "./title-bar.css";
import "./settings-flyout.css";
import { startAccent } from "./accent";
import { startBackdrop } from "./backdrop";
import { startCleanCopy } from "./clean-copy";
import { startCommands } from "./commands";
import { startSelectionCounts } from "./counts";
import { startTaggingDrags } from "./drags";
import { startFind } from "./find";
import { startFullScreen } from "./full-screen";
import { startKeepOnTop } from "./keep-on-top";
import { startLayout } from "./layout";
import { startLinks } from "./links";
import { startSettings } from "./settings";
import { startSettingsFlyout } from "./settings-flyout";
import { epochMs, watchLongTasks } from "./timing";
import { startTitleBar } from "./title-bar";
import { startDocumentView } from "./view";

const scriptStart = epochMs();
// First, so every part of the page starts with the settings in the page's URL.
const hearingSettings = startSettings();
startBackdrop();
// Before the first paint, so a reader who chose Windows' accent color never sees amber first.
startAccent();
watchLongTasks();
// Before the title bar draws Keep on top's button.
startKeepOnTop();
startFullScreen();
startTitleBar();
// After the title bar, whose gear it opens under.
startSettingsFlyout();
startCommands();
startCleanCopy();
startTaggingDrags();
startLinks();
startSelectionCounts();
startFind();
// Before the document view starts, so the first paint is at the zoom level and width already.
startLayout();
// The document view asks for its document only once the page hears the settings: the app holds a
// change made before the first request and sends it on that request (ticket 16's review).
await hearingSettings;
await startDocumentView(scriptStart);
