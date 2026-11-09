// Measures the release build against the spec's performance targets with the timing switch
// (ticket 02): run `npm run tauri build`, then `npm run measure`. Each case starts a new process
// on the test documents, reads the app's timing reports from stderr, and gives the median of
// five runs. One run before each case is discarded, so the first start after a build, when
// Windows scans the new executable, doesn't count.

import { execFileSync, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { performance } from "node:perf_hooks";
import { setTimeout as sleep } from "node:timers/promises";
import { DOCUMENTS, writeDocuments } from "./generate-documents.js";

const EXECUTABLE = join(
  import.meta.dirname,
  "..",
  "target",
  "release",
  "downlight.exe",
);
const RUNS = 5;
const RUN_TIMEOUT_MS = 20_000;
// Long tasks and frames after first paint, such as layout that a large document leaves for
// later, still count; this is how long a run keeps listening after the last first paint.
const SETTLE_MS = 3_000;
// The scrolling cases drive the window over WebView2's debug port.
const DEBUG_PORT = 9333;

/**
 * Starts Downlight with the timing switch on and collects its reports until its windows painted.
 * Each of `laterDocuments` is then opened by a further launch, which hands it to the running
 * process (ADR 0001), and the run waits for its window to paint too. With `scroll`, it then
 * scrolls through the first window's document.
 */
async function run(documents, { scroll = false, laterDocuments = [] } = {}) {
  const env = { ...process.env, DOWNLIGHT_TIMING: "1" };
  if (scroll) {
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
    const deadline = Date.now() + RUN_TIMEOUT_MS;
    const painted = async (count) => {
      // A window counts once it has painted and been shown.
      const done = (event) => reports.filter((r) => r.event === event).length;
      while (done("first-paint") < count || done("window-shown") < count) {
        if (Date.now() > deadline)
          throw new Error(
            `No first paint or shown window within ${RUN_TIMEOUT_MS} ms`,
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
    if (scroll) await scrollThrough();
    await sleep(SETTLE_MS);
    return { reports, memory: memoryOf(child.pid), launches };
  } finally {
    execFileSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
      stdio: "ignore",
    });
    // Lets WebView2 release its user data folder before the next run starts.
    await sleep(1_000);
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
 * Jumps to the end and to the middle, then pages down 150 times, one page a frame: the moves
 * that lay out blocks the reader hasn't seen yet.
 */
async function scrollThrough() {
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
  const evaluate = (expression) =>
    new Promise((resolve) => {
      const id = ++lastId;
      socket.addEventListener("message", function answered(event) {
        if (JSON.parse(event.data).id !== id) return;
        socket.removeEventListener("message", answered);
        resolve();
      });
      socket.send(
        JSON.stringify({
          id,
          method: "Runtime.evaluate",
          params: { expression, awaitPromise: true },
        }),
      );
    });
  try {
    await evaluate("document.scrollingElement.scrollTop = 1e9");
    await sleep(1_000);
    await evaluate(
      "document.scrollingElement.scrollTop = document.scrollingElement.scrollHeight / 2",
    );
    await sleep(1_000);
    await evaluate(`(async () => {
      const view = document.scrollingElement;
      view.scrollTop = 0;
      for (let i = 0; i < 150; i++) {
        view.scrollTop += innerHeight;
        await new Promise((frame) => requestAnimationFrame(frame));
      }
    })()`);
  } finally {
    socket.close();
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

/** Runs a case RUNS times after one discarded run and returns the measures of each run. */
async function measure(name, documents, measures, options) {
  console.error(`${name}: warming up`);
  await run(documents, options);
  const runs = [];
  for (let i = 1; i <= RUNS; i++) {
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

function table(cases) {
  const keys = [...new Set(cases.flatMap((c) => Object.keys(c.runs[0])))];
  const lines = [
    `| Measure (median of ${RUNS}) | ${cases.map((c) => c.name).join(" | ")} |`,
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
const scrolling = [
  await measure("1 MB mixed", [DOCUMENTS.mixed1mb], scrolled, {
    scroll: true,
  }),
  await measure("1 MB prose", [DOCUMENTS.prose1mb], scrolled, {
    scroll: true,
  }),
];
console.log(table(cases));
console.log();
console.log(table([second]));
console.log();
console.log("Scrolling through the document after first paint:");
console.log();
console.log(table(scrolling));
