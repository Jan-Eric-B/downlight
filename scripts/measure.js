// Measures the release build against the spec's performance targets with the timing switch
// (ticket 02): run `npm run tauri build`, then `npm run measure`. Each case starts a new process
// on the test documents, reads the app's timing reports from stderr, and gives the median of
// five runs. One run before each case is discarded, so the first start after a build, when
// Windows scans the new executable, doesn't count. `npm run measure -- large` measures the 1 MB
// documents and the large ones instead (ticket 31).

import { execFileSync, spawn } from "node:child_process";
import {
  appendFileSync,
  copyFileSync,
  existsSync,
  readFileSync,
} from "node:fs";
import { join } from "node:path";
import { performance } from "node:perf_hooks";
import { setTimeout as sleep } from "node:timers/promises";
import {
  DOCUMENTS,
  LARGE_DOCUMENTS,
  writeDocuments,
} from "./generate-documents.js";

const EXECUTABLE = join(
  import.meta.dirname,
  "..",
  "target",
  "release",
  "downlight.exe",
);
const RUNS = 5;
const RUN_TIMEOUT_MS = 20_000;
// The large documents (`npm run measure -- large`): fewer runs, and longer to wait for each.
const LARGE_RUNS = 3;
const LARGE_RUN_TIMEOUT_MS = 180_000;
// Long tasks and frames after first paint, such as layout that a large document leaves for
// later, still count; this is how long a run keeps listening after the last first paint.
const SETTLE_MS = 3_000;
// The scrolling and reload cases drive the window over WebView2's debug port.
const DEBUG_PORT = 9333;
// The reload case's document: a copy of the 1 MB one, since the run appends to it.
const RELOAD_DOCUMENT = DOCUMENTS.mixed1mb.replace(/\.md$/, "-reload.md");

/**
 * Starts Downlight with the timing switch on and collects its reports until its windows painted.
 * Each of `laterDocuments` is then opened by a further launch, which hands it to the running
 * process (ADR 0001), and the run waits for its window to paint too. `afterPaint`, if given, then
 * acts on the first window over WebView2's debug port (`scrollThrough`, `reloadInTheMiddle` or
 * `typeInFind`), and the run returns its result as `after`. `prepare` runs before the process
 * starts. A run ends once its processes are gone, so the next one starts on none.
 */
async function run(
  documents,
  { afterPaint, laterDocuments = [], prepare, timeoutMs = RUN_TIMEOUT_MS } = {},
) {
  prepare?.();
  const env = { ...process.env, DOWNLIGHT_TIMING: "1" };
  if (afterPaint) {
    env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = `--remote-debugging-port=${DEBUG_PORT}`;
  }
  const child = spawn(EXECUTABLE, documents, {
    env,
    stdio: ["ignore", "ignore", "pipe"],
  });
  const reports = [];
  let buffered = "";
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => {
    buffered += chunk;
    const lines = buffered.split(/\r?\n/);
    buffered = lines.pop() ?? "";
    for (const line of lines) {
      const report = parse(line);
      // When the script saw it, for measures that start in the script, such as a launch.
      if (report) reports.push({ ...report, seen: performance.now() });
    }
  });
  try {
    const deadline = Date.now() + timeoutMs;
    const painted = async (count) => {
      // A window counts once it has painted and been shown.
      const done = (event) => reports.filter((r) => r.event === event).length;
      while (done("first-paint") < count || done("window-shown") < count) {
        if (Date.now() > deadline)
          throw new Error(
            `No first paint or shown window within ${timeoutMs} ms`,
          );
        if (child.exitCode !== null)
          throw new Error(`Downlight exited with ${child.exitCode}`);
        await sleep(20);
      }
    };
    await painted(documents.length);
    const launches = [];
    for (const document of laterDocuments) {
      launches.push(await launch(document, env));
    }
    await painted(documents.length + laterDocuments.length);
    const after = await afterPaint?.({ documents, reports, deadline });
    await sleep(SETTLE_MS);
    return { reports, memory: memoryOf(child.pid), launches, after };
  } finally {
    // taskkill fails when a process in the tree exits while it ends them, which WebView2's do
    // once their browser process is gone, so its failure says nothing: the wait below checks.
    try {
      execFileSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
        stdio: "ignore",
      });
    } catch {
      // Checked by `untilNoDownlight`.
    }
    await untilNoDownlight();
  }
}

/** Downlight's user data folder for WebView2 is named after the app's identifier. */
const APP_IDENTIFIER = JSON.parse(
  readFileSync(
    join(import.meta.dirname, "..", "app", "tauri.conf.json"),
    "utf8",
  ),
).identifier;

/** How long to wait for Downlight and its WebView2 processes to be gone. */
const GONE_TIMEOUT_MS = 60_000;

/**
 * Every Downlight process, and every WebView2 process on Downlight's user data folder, whoever
 * started it: another Downlight process would take a run's launch as a second launch (ADR 0001),
 * and a WebView2 process still on the folder, such as one shutting down after a run was ended,
 * makes the next window fail with "The requested resource is in use" in an error box that waits
 * for a click.
 */
function downlightProcesses() {
  const json = execFileSync(
    "powershell",
    [
      "-NoProfile",
      "-Command",
      "Get-CimInstance Win32_Process -Filter \"Name = 'downlight.exe' OR Name = 'msedgewebview2.exe'\" | Select-Object ProcessId,Name,CommandLine | ConvertTo-Json -Compress",
    ],
    { encoding: "utf8" },
  ).trim();
  const processes = json === "" ? [] : [JSON.parse(json)].flat();
  return processes.filter(
    (p) =>
      p.Name.toLowerCase() === "downlight.exe" ||
      (p.CommandLine ?? "").includes(APP_IDENTIFIER),
  );
}

/**
 * Waits until no Downlight process and no WebView2 process on its user data folder is left
 * (`downlightProcesses`). If one stays, the measurement stops rather than start a run that would
 * hand its documents to that process or fail with an error box; it ends no process it didn't
 * start, since one left may be the reader's or another build's.
 */
async function untilNoDownlight() {
  const deadline = Date.now() + GONE_TIMEOUT_MS;
  for (;;) {
    const left = downlightProcesses();
    if (left.length === 0) return;
    if (Date.now() > deadline) {
      console.error(
        `Downlight processes still running after ${GONE_TIMEOUT_MS} ms: ${left.map((p) => `${p.Name} ${p.ProcessId}`).join(", ")}. Close them, then measure again.`,
      );
      process.exit(1);
    }
    await sleep(500);
  }
}

/** Runs a further launch on `document`: when it started, and the ms until it exited. */
async function launch(document, env) {
  const started = performance.now();
  const child = spawn(EXECUTABLE, [document], { env, stdio: "ignore" });
  await new Promise((resolve, reject) => {
    child.on("exit", resolve);
    child.on("error", reject);
  });
  return { started, returned: performance.now() - started };
}

/**
 * Connects to the first window's page over WebView2's debug port. `send` calls a DevTools
 * protocol method and resolves with its result; `evaluate` runs an expression in the page, waits
 * for a promise it gives, and resolves with its value.
 */
async function connect() {
  const targets = await (
    await fetch(`http://127.0.0.1:${DEBUG_PORT}/json`)
  ).json();
  const page = targets.find((t) => t.type === "page");
  const socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = reject;
  });
  let lastId = 0;
  const send = (method, params) =>
    new Promise((resolve) => {
      const id = ++lastId;
      socket.addEventListener("message", function answered(event) {
        const message = JSON.parse(event.data);
        if (message.id !== id) return;
        socket.removeEventListener("message", answered);
        resolve(message.result);
      });
      socket.send(JSON.stringify({ id, method, params }));
    });
  const evaluate = async (expression) =>
    (
      await send("Runtime.evaluate", {
        expression,
        awaitPromise: true,
        returnByValue: true,
      })
    )?.result?.value;
  return { send, evaluate, close: () => socket.close() };
}

/** What the reader types into the find bar, one character at a time; the last matches nothing. */
const FIND_QUERIES = ["e", "the", "lamplight", "qxzjv"];

/**
 * In the page: the element the document scrolls in, below the title bar. The selector is
 * `SCROLLER_SELECTOR` in `page/src/view.ts`, which this script, run in Node, can't import.
 */
const SCROLLER = `document.querySelector(".view-scroller")`;

/** Resolves in the page once the next frame is done, as the page's `afterNextFrame` does. */
const NEXT_FRAME =
  "new Promise((done) => requestAnimationFrame(() => setTimeout(done, 0)))";

/**
 * In the page: resolves with the time from `start` to the end of the first frame that shows the
 * find bar's count. Until the document's text is indexed the bar shows none, and a keystroke
 * before then counts until it does.
 */
const AFTER_COUNT = `(start) => new Promise((done) => {
  const status = document.querySelector(".find-bar [role=status]");
  const check = () => {
    if (status.textContent === "") requestAnimationFrame(check);
    else setTimeout(() => done(performance.now() - start), 0);
  };
  requestAnimationFrame(check);
})`;

/**
 * Opens the find bar with Ctrl+F and types each of `FIND_QUERIES` into it, one character at a
 * time, each once the last one's count is shown; Ctrl+F before each selects the box's text, so
 * the query's first character replaces it. Returns each keystroke's time from its `input` event
 * to the end of the first frame that shows its count, and when the typing started and ended.
 */
async function typeInFind() {
  const { send, evaluate, close } = await connect();
  try {
    // Before the page's own listener, which is on the box: the window's capture phase comes
    // first.
    await evaluate(`(() => {
      window.findKeystrokes = [];
      window.findCounted = Promise.resolve();
      const afterCount = ${AFTER_COUNT};
      addEventListener("input", () => {
        findCounted = afterCount(performance.now()).then((time) => findKeystrokes.push(time));
      }, true);
    })()`);
    const ctrlF = async () => {
      const key = {
        modifiers: 2,
        key: "f",
        code: "KeyF",
        windowsVirtualKeyCode: 70,
      };
      await send("Input.dispatchKeyEvent", { type: "rawKeyDown", ...key });
      await send("Input.dispatchKeyEvent", { type: "keyUp", ...key });
      await evaluate(NEXT_FRAME);
    };
    const started = performance.now();
    for (const query of FIND_QUERIES) {
      await ctrlF();
      for (const character of query) {
        await send("Input.insertText", { text: character });
        await evaluate("findCounted");
      }
    }
    // The last keystroke's count is shown by now, as the wait after it was.
    const keystrokes = await evaluate("findKeystrokes");
    return { keystrokes, started, ended: performance.now() };
  } finally {
    close();
  }
}

/**
 * The line number of the first top-level block not wholly above the top of the view, for
 * checking that a reload kept the reading position.
 */
const TOP_LINE = `(() => {
  const viewTop = ${SCROLLER}.getBoundingClientRect().top;
  const blocks = [...document.querySelectorAll("article > [data-sourcepos], article > .highlight > pre")];
  const top = blocks.find((b) => b.getBoundingClientRect().bottom > viewTop);
  return top && Number.parseInt(top.getAttribute("data-sourcepos"));
})()`;

/**
 * Scrolls to the middle of the document and appends a line to its file, as a save in an editor
 * does. Returns when the file was saved, and the line at the top of the view before the save.
 */
async function saveInTheMiddle(document) {
  const { evaluate, close } = await connect();
  try {
    await evaluate(`(() => {
      const view = ${SCROLLER};
      view.scrollTop = view.scrollHeight / 2;
    })()`);
    await sleep(1_000);
    const lineBefore = await evaluate(TOP_LINE);
    const at = performance.now();
    appendFileSync(document, "\nA line added by the reload measurement.\n");
    return { at, lineBefore };
  } finally {
    close();
  }
}

/**
 * Saves the file in the middle of the first window's document (`saveInTheMiddle`), waits for
 * the reload to be shown and for the page to settle, then reads the line at the top of the view.
 * Returns when the file was saved and probed, and the line before and after.
 */
async function reloadInTheMiddle({ documents, reports, deadline }) {
  const saved = await saveInTheMiddle(documents[0]);
  while (!reports.some((r) => r.event === "reload-shown")) {
    if (Date.now() > deadline)
      throw new Error(`No reload shown within ${RUN_TIMEOUT_MS} ms`);
    await sleep(20);
  }
  // The probe comes after the settle time, since its own layout reads take long frames of their
  // own, which the measures leave out.
  await sleep(SETTLE_MS);
  const probed = performance.now();
  return { ...saved, probed, lineAfter: await topLineNow() };
}

/** The line at the top of the view now (`TOP_LINE`). */
async function topLineNow() {
  const { evaluate, close } = await connect();
  try {
    return await evaluate(TOP_LINE);
  } finally {
    close();
  }
}

/**
 * Jumps to the end and to the middle, then pages down 150 times, one page a frame: the moves
 * that lay out blocks the reader hasn't seen yet.
 */
async function scrollThrough() {
  const { evaluate, close } = await connect();
  try {
    await evaluate(`${SCROLLER}.scrollTop = 1e9`);
    await sleep(1_000);
    await evaluate(`(() => {
      const view = ${SCROLLER};
      view.scrollTop = view.scrollHeight / 2;
    })()`);
    await sleep(1_000);
    await evaluate(`(async () => {
      const view = ${SCROLLER};
      view.scrollTop = 0;
      for (let i = 0; i < 150; i++) {
        view.scrollTop += view.clientHeight;
        await new Promise((frame) => requestAnimationFrame(frame));
      }
    })()`);
  } finally {
    close();
  }
}

/** `timing <window> <event> key=value ...` */
function parse(line) {
  const [tag, window, event, ...pairs] = line.trim().split(" ");
  if (tag !== "timing" || !window || !event) return undefined;
  const fields = Object.fromEntries(
    pairs.map((pair) => {
      const [key, value] = pair.split("=");
      return [key, Number(value)];
    }),
  );
  return { window, event, ...fields };
}

/** The working set and private bytes, in MB, of Downlight and every process it started. */
function memoryOf(pid) {
  const json = execFileSync(
    "powershell",
    [
      "-NoProfile",
      "-Command",
      "Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,WorkingSetSize,PrivatePageCount | ConvertTo-Json -Compress",
    ],
    { encoding: "utf8" },
  );
  const processes = JSON.parse(json);
  const tree = new Set([pid]);
  for (let grew = true; grew;) {
    grew = false;
    for (const p of processes) {
      if (tree.has(p.ParentProcessId) && !tree.has(p.ProcessId)) {
        tree.add(p.ProcessId);
        grew = true;
      }
    }
  }
  const inTree = processes.filter((p) => tree.has(p.ProcessId));
  const mb = (bytes) => bytes / 1024 / 1024;
  return {
    processes: inTree.length,
    workingSet: mb(
      inTree.reduce((sum, p) => sum + Number(p.WorkingSetSize), 0),
    ),
    privateBytes: mb(
      inTree.reduce((sum, p) => sum + Number(p.PrivatePageCount), 0),
    ),
  };
}

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2
    ? sorted[middle]
    : (sorted[middle - 1] + sorted[middle]) / 2;
}

/**
 * Runs a case `runs` times (RUNS unless the options say) after one discarded run and returns the
 * measures of each run.
 */
async function measure(name, documents, measures, options = {}) {
  console.error(`${name}: warming up`);
  await run(documents, options);
  const runs = [];
  for (let i = 1; i <= (options.runs ?? RUNS); i++) {
    const result = await run(documents, options);
    const values = measures(result);
    console.error(`${name}: run ${i}: ${JSON.stringify(values)}`);
    runs.push(values);
  }
  return { name, runs };
}

function reportsOf(reports, label, event) {
  return reports.filter((r) => r.window === label && r.event === event);
}

/**
 * The longest task or frame before the page received the document, and from then on: the page's
 * own load comes before, and everything the document's size causes after.
 */
function longest(reports, paint) {
  const durations = (keep) =>
    reports
      .filter((r) => r.event === "long-task" || r.event === "long-frame")
      .filter(keep)
      .map((r) => r.duration);
  return {
    longestBeforeDocument: Math.max(
      0,
      ...durations((r) => r.at < paint.received),
    ),
    longestAfterDocument: Math.max(
      0,
      ...durations((r) => r.at >= paint.received),
    ),
  };
}

/** What every case reports about a window: read and render, long tasks and memory. */
function windowMeasures(reports, memory, label) {
  const [paint] = reportsOf(reports, label, "first-paint");
  const [readAndRender] = reportsOf(reports, label, "read-and-render");
  return {
    paint,
    readAndRender,
    measures: {
      readAndRender: readAndRender.read + readAndRender.render,
      ...longest(
        reports.filter((r) => r.window === label),
        paint,
      ),
      workingSetMb: memory.workingSet,
      privateMb: memory.privateBytes,
    },
  };
}

/** First paint of the first window, the stages before it, and memory. */
function firstWindow({ reports, memory }) {
  const { paint, readAndRender, measures } = windowMeasures(
    reports,
    memory,
    "document-0",
  );
  const [opening] = reportsOf(reports, "document-0", "window-opening");
  const [shown] = reportsOf(reports, "document-0", "window-shown");
  return {
    firstPaint: paint.at,
    // The window opens hidden and shows after its first frame (ticket 04), so the reader sees
    // the document from here.
    shown: shown.at,
    windowOpening: opening.at,
    navigation: paint.navigation,
    script: paint.script,
    readStart: readAndRender.at,
    received: paint.received,
    inserted: paint.inserted,
    ...measures,
    processes: memory.processes,
  };
}

/**
 * Where a large document's time goes (ticket 31): the app's read and render, sending the HTML to
 * the page (from the render's end until the page has it), inserting it, and the frame after, to
 * first paint. The window is shown at first paint, or before it with the wait note when the
 * document takes over half a second to arrive.
 */
function largeDocument({ reports, memory }) {
  const [shown] = reportsOf(reports, "document-0", "window-shown");
  const [paint] = reportsOf(reports, "document-0", "first-paint");
  const [readAndRender] = reportsOf(reports, "document-0", "read-and-render");
  const longestTask = Math.max(
    0,
    ...reports
      .filter((r) => r.event === "long-task" || r.event === "long-frame")
      .map((r) => r.duration),
  );
  return {
    read: readAndRender.read,
    render: readAndRender.render,
    // The render starts ahead, while the webview starts, so a small one is done before the page
    // asks; the HTML is on its way from the later of the two.
    ipc:
      paint.received -
      Math.max(
        paint.requested,
        readAndRender.at + readAndRender.read + readAndRender.render,
      ),
    insert: paint.inserted - paint.received,
    frameAfterInsert: paint.at - paint.inserted,
    shown: shown.at,
    firstPaint: paint.at,
    longestTask,
    workingSetMb: memory.workingSet,
    privateMb: memory.privateBytes,
  };
}

/**
 * `npm run measure -- large`: the 1 MB documents and the large ones (ticket 31), with three runs
 * each beyond 1 MB, since a 100 MB run takes a while. A case that fails, by a crash or no window
 * shown in time, is reported as failed rather than stopping the rest.
 */
async function measureLarge() {
  const attempt = async (name, document, runs) => {
    try {
      return await measure(name, [document], largeDocument, {
        timeoutMs: LARGE_RUN_TIMEOUT_MS,
        runs,
      });
    } catch (error) {
      console.error(`${name}: failed: ${error.message}`);
      return { name, failed: error.message };
    }
  };
  const results = [];
  for (const kind of ["mixed", "prose"]) {
    const oneMb = kind === "mixed" ? DOCUMENTS.mixed1mb : DOCUMENTS.prose1mb;
    results.push(await attempt(`1 MB ${kind}`, oneMb, RUNS));
    for (const large of LARGE_DOCUMENTS) {
      results.push(
        await attempt(`${large.mb} MB ${kind}`, large[kind], LARGE_RUNS),
      );
    }
  }
  const measured = results.filter((r) => !r.failed);
  console.log(
    "Large documents, ms since the process started for firstPaint and shown:",
  );
  console.log();
  if (measured.length > 0) console.log(table(measured));
  for (const { name, failed } of results.filter((r) => r.failed))
    console.log(`- ${name}: failed: ${failed}`);
}

function secondWindow({ reports, memory, launches }) {
  const { paint, measures } = windowMeasures(reports, memory, "document-1");
  const [opening] = reportsOf(reports, "document-1", "window-opening");
  const [shown] = reportsOf(reports, "document-1", "window-shown");
  return {
    secondLaunchReturns: launches[0].returned,
    // What the reader waits for: the hand-off to the running process and the new window.
    firstPaintSinceLaunch: paint.seen - launches[0].started,
    shownSinceLaunch: shown.seen - launches[0].started,
    firstPaintSinceWindow: paint.at - opening.at,
    shownSinceWindow: shown.at - opening.at,
    ...measures,
  };
}

function scrolled({ reports, memory }) {
  const { measures } = windowMeasures(reports, memory, "document-0");
  return { longestAfterDocument: measures.longestAfterDocument };
}

/** A save to the file in the middle of the 1 MB document, until the reload is shown. */
function reloaded({ reports, after: saved }) {
  const [reload] = reportsOf(reports, "document-0", "reload");
  const [shown] = reportsOf(reports, "document-0", "reload-shown");
  const after = reports
    .filter((r) => r.event === "long-task" || r.event === "long-frame")
    .filter((r) => r.at >= reload.at && r.seen < saved.probed)
    .map((r) => r.duration);
  return {
    // The 150 ms of quiet the watch waits for are part of this.
    shownSinceSave: shown.seen - saved.at,
    read: reload.read,
    render: reload.render,
    insert: shown.insert,
    anchor: shown.anchor,
    longestAfterReload: Math.max(0, ...after),
    // How far the line at the top of the view moved; 0 when the reload kept the reading position.
    linesMoved: Math.abs(saved.lineAfter - saved.lineBefore),
  };
}

/** Typing into the find bar: each keystroke until its count is shown, and the longest task. */
function typed({ reports, after: found }) {
  const during = reports
    .filter((r) => r.event === "long-task" || r.event === "long-frame")
    // A report reaches the script a little after its task, so it counts until the run's end.
    .filter((r) => r.seen >= found.started)
    .map((r) => r.duration);
  const [first, ...later] = found.keystrokes;
  return {
    keystrokeToCount: median(found.keystrokes),
    // The first keystroke can wait for the text index that Ctrl+F started, so it is reported apart.
    firstKeystrokeToCount: first,
    longestLaterKeystrokeToCount: Math.max(...later),
    longestWhileTyping: Math.max(0, ...during),
  };
}

function table(cases) {
  const keys = [...new Set(cases.flatMap((c) => Object.keys(c.runs[0])))];
  const lines = [
    `| Measure (median of ${[...new Set(cases.map((c) => c.runs.length))].join(" or ")}) | ${cases.map((c) => c.name).join(" | ")} |`,
    `| --- |${cases.map(() => " ---: |").join("")}`,
  ];
  for (const key of keys) {
    const cells = cases.map((c) => {
      if (!(key in c.runs[0])) return "";
      const values = c.runs.map((r) => r[key]);
      const cell = median(values).toFixed(key === "processes" ? 0 : 1);
      // The target for these is "never", so the worst run counts too.
      return key.startsWith("longest")
        ? `${cell} (worst ${Math.max(...values).toFixed(1)})`
        : cell;
    });
    lines.push(`| ${key} | ${cells.join(" | ")} |`);
  }
  return lines.join("\n");
}

if (!existsSync(EXECUTABLE)) {
  console.error(`No ${EXECUTABLE}. Run npm run tauri build first.`);
  process.exit(1);
}
// Each run waits for its own processes to be gone; this is for any left from before.
await untilNoDownlight();
if (process.argv.includes("large")) {
  writeDocuments({ large: true });
  await measureLarge();
  process.exit(0);
}
writeDocuments();
const cases = [
  await measure("100 KB", [DOCUMENTS.mixed100kb], firstWindow),
  await measure("1 MB mixed", [DOCUMENTS.mixed1mb], firstWindow),
  await measure("1 MB prose", [DOCUMENTS.prose1mb], firstWindow),
];
const second = await measure(
  "Second window, 100 KB",
  [DOCUMENTS.mixed100kb],
  secondWindow,
  { laterDocuments: [DOCUMENTS.mixed100kbOther] },
);
const reload = await measure(
  "Reload, 1 MB mixed",
  [RELOAD_DOCUMENT],
  reloaded,
  // A fresh copy each run, since a run appends to it.
  {
    afterPaint: reloadInTheMiddle,
    prepare: () => copyFileSync(DOCUMENTS.mixed1mb, RELOAD_DOCUMENT),
  },
);
const scrolling = [
  await measure("1 MB mixed", [DOCUMENTS.mixed1mb], scrolled, {
    afterPaint: scrollThrough,
  }),
  await measure("1 MB prose", [DOCUMENTS.prose1mb], scrolled, {
    afterPaint: scrollThrough,
  }),
];
const finding = [
  await measure("1 MB mixed", [DOCUMENTS.mixed1mb], typed, {
    afterPaint: typeInFind,
  }),
  await measure("1 MB prose", [DOCUMENTS.prose1mb], typed, {
    afterPaint: typeInFind,
  }),
];
console.log(table(cases));
console.log();
console.log(table([second]));
console.log();
console.log("Scrolling through the document after first paint:");
console.log();
console.log(table(scrolling));
console.log();
console.log("A save in the middle of the document, until the reload is shown:");
console.log();
console.log(table([reload]));
console.log();
console.log(
  `Typing ${FIND_QUERIES.map((q) => `"${q}"`).join(", ")} into the find bar, each character once the last one's count is shown:`,
);
console.log();
console.log(table(finding));
