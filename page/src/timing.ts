import { invoke } from "@tauri-apps/api/core";

// The page's half of the timing switch (DOWNLIGHT_TIMING): the app opens the page with
// `?timing` when the switch is on, and the app writes the reports out.

const isOn = new URLSearchParams(location.search).has("timing");

// A report that can't reach the app has nowhere else to go; `npm run measure` then stops with
// "No first paint or shown window", which is where the person measuring sees it.
const send = (command: string, args: Record<string, unknown>) => {
  invoke(command, args).catch(() => undefined);
};

/** Tasks and frames longer than this keep the window from answering too long (the spec's target). */
const LONG_TASK_MS = 100;

/** Now, in ms since the Unix epoch, so the app can compare it with its own clock. */
export function epochMs(): number {
  return performance.timeOrigin + performance.now();
}

/**
 * Reports every page task, and every frame, that takes longer than 100 ms. Frames count
 * separately because a task report misses the style, layout and paint after a large insert.
 */
export function watchLongTasks(): void {
  if (!isOn) return;
  const report = (kind: "task" | "frame", entry: PerformanceEntry) => {
    if (entry.duration <= LONG_TASK_MS) return;
    send("report_long_task", {
      task: {
        kind,
        start: performance.timeOrigin + entry.startTime,
        duration: entry.duration,
      },
    });
  };
  const watch = (type: string, kind: "task" | "frame") => {
    if (!PerformanceObserver.supportedEntryTypes.includes(type)) return;
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) report(kind, entry);
    }).observe({ type, buffered: true });
  };
  watch("longtask", "task");
  watch("long-animation-frame", "frame");
}

/** The marks on the way to first paint, each from `epochMs()`. */
interface FirstPaintMarks {
  scriptStart: number;
  documentRequested: number;
  documentReceived: number;
  documentInserted: number;
  /** When the frame after the document was inserted was done. */
  firstFrame: number;
}

export function reportFirstPaint(marks: FirstPaintMarks): void {
  if (!isOn) return;
  send("report_first_paint", {
    marks: { ...marks, navigationStart: performance.timeOrigin },
  });
}

/** The marks while a reload is shown, each from `epochMs()`. */
interface ReloadMarks {
  /** When the reloaded document's HTML arrived. */
  received: number;
  inserted: number;
  /** When the view was back at the reading position. */
  anchored: number;
}

export function reportReload(marks: ReloadMarks): void {
  if (!isOn) return;
  send("report_reload", { marks });
}
