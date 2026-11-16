import {
  askApp,
  pageFlags,
  type FirstPaintMarks,
  type LongTask,
  type ReloadMarks,
} from "./app";

// The page's half of the timing switch (DOWNLIGHT_TIMING): the app opens the page with the
// `timing` flag when the switch is on, and the app writes the reports out.

const isOn = pageFlags().timing;

// A report that can't reach the app has nowhere else to go; `npm run measure` then stops with
// "No first paint or shown window", which is where the person measuring sees it.
const send = (report: Promise<null>) => {
  report.catch(() => undefined);
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
  const report = (kind: LongTask["kind"], entry: PerformanceEntry) => {
    if (entry.duration <= LONG_TASK_MS) return;
    send(
      askApp("report_long_task", {
        task: {
          kind,
          start: performance.timeOrigin + entry.startTime,
          duration: entry.duration,
        },
      }),
    );
  };
  const watch = (type: string, kind: LongTask["kind"]) => {
    if (!PerformanceObserver.supportedEntryTypes.includes(type)) return;
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) report(kind, entry);
    }).observe({ type, buffered: true });
  };
  watch("longtask", "task");
  watch("long-animation-frame", "frame");
}

/** The marks the page makes on the way to first paint, each from `epochMs()`. */
type PageMarks = Omit<FirstPaintMarks, "navigationStart">;

/** First paint's marks so far, each made where it happens. */
const firstPaint: Partial<PageMarks> = {};

let firstPaintReported = false;

/**
 * Makes first paint's `mark`, now or at `at`, until first paint is reported. A mark made again
 * replaces the one before: when a later request overtakes the first, the reader waits for the
 * later one's document.
 */
export function markFirstPaint(mark: keyof PageMarks, at = epochMs()): void {
  if (!isOn || firstPaintReported) return;
  firstPaint[mark] = at;
}

/** Reports first paint with its marks, once. */
export function reportFirstPaint(): void {
  if (!isOn || firstPaintReported) return;
  firstPaintReported = true;
  const {
    scriptStart,
    documentRequested,
    documentReceived,
    documentInserted,
    firstFrame,
  } = firstPaint;
  // Each is made on the way to the frame that reports, so a missing one is a page bug, and the
  // missing report makes `npm run measure` say so.
  if (
    scriptStart === undefined ||
    documentRequested === undefined ||
    documentReceived === undefined ||
    documentInserted === undefined ||
    firstFrame === undefined
  )
    return;
  const marks: FirstPaintMarks = {
    navigationStart: performance.timeOrigin,
    scriptStart,
    documentRequested,
    documentReceived,
    documentInserted,
    firstFrame,
  };
  send(askApp("report_first_paint", { marks }));
}

/** Reports a reload with its marks, each from `epochMs()`. */
export function reportReload(marks: ReloadMarks): void {
  if (!isOn) return;
  send(askApp("report_reload", { marks }));
}
