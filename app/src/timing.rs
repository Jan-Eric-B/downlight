//! The timing switch. With the environment variable `DOWNLIGHT_TIMING` set, the app reports on
//! stderr how long each window takes to its first paint, the stages on the way there, and every
//! page task or frame longer than 100 ms. It is off by default. Work that grows with a document's
//! size is measured with it (CODING_STANDARDS.md), and `npm run measure` runs it on the test
//! documents.
//!
//! Every report is one line, `timing <window> <event> key=value ...`. Points in time are in ms
//! since the process started; durations are in ms. All points share that one clock, so a reader
//! of the reports finds the time between two events by subtracting them, as `npm run measure`
//! does for a further window's cost from its `window-opening`.

use std::io::Write as _;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use tauri::{State, WebviewWindow};

use crate::platform;

const SWITCH: &str = "DOWNLIGHT_TIMING";

/// Whether the timing switch is on, and the process start that reports count from.
pub struct Timing {
    /// In ms since the Unix epoch; `None` while the switch is off.
    process_start_ms: Option<f64>,
}

/// When reading and rendering a document started, and how long each took.
pub struct ReadAndRenderTimes {
    pub started: SystemTime,
    pub read: Duration,
    pub render: Duration,
}

impl Timing {
    /// Reads the switch from the environment. Call it first thing in `main`: without the OS's
    /// process start time, timing counts from this call.
    pub fn from_env() -> Self {
        let process_start_ms = std::env::var_os(SWITCH)
            .map(|_| epoch_ms_of(platform::process_start().unwrap_or_else(SystemTime::now)));
        Self { process_start_ms }
    }

    pub fn is_on(&self) -> bool {
        self.process_start_ms.is_some()
    }

    /// Notes that a window starts opening, which is where a further window in a running process
    /// starts to cost the reader.
    pub fn window_opening(&self, window: &str) {
        self.report_now(window, "window-opening");
    }

    /// Notes that a window was shown. It opens hidden and shows once its page has drawn its first
    /// frame, so for the reader first paint is not before this.
    pub fn window_shown(&self, window: &str) {
        self.report_now(window, "window-shown");
    }

    pub fn read_and_render(&self, window: &str, times: &ReadAndRenderTimes) {
        let Some(start) = self.process_start_ms else {
            return;
        };
        report(
            window,
            "read-and-render",
            &[
                ("at", epoch_ms_of(times.started) - start),
                ("read", times.read.as_secs_f64() * 1000.0),
                ("render", times.render.as_secs_f64() * 1000.0),
            ],
        );
    }

    fn first_paint(&self, window: &str, marks: &FirstPaintMarks) {
        let Some(start) = self.process_start_ms else {
            return;
        };
        report(
            window,
            "first-paint",
            &[
                ("at", marks.first_frame - start),
                ("navigation", marks.navigation_start - start),
                ("script", marks.script_start - start),
                ("requested", marks.document_requested - start),
                ("received", marks.document_received - start),
                ("inserted", marks.document_inserted - start),
            ],
        );
    }

    fn long_task(&self, window: &str, task: &LongTask) {
        let Some(start) = self.process_start_ms else {
            return;
        };
        report(
            window,
            task.kind.event(),
            &[("at", task.start - start), ("duration", task.duration)],
        );
    }

    fn report_now(&self, window: &str, event: &str) {
        let Some(start) = self.process_start_ms else {
            return;
        };
        report(
            window,
            event,
            &[("at", epoch_ms_of(SystemTime::now()) - start)],
        );
    }
}

fn report(window: &str, event: &str, fields: &[(&str, f64)]) {
    let mut line = format!("timing {window} {event}");
    for (key, value) in fields {
        line.push_str(&format!(" {key}={value:.1}"));
    }
    // A report that can't be written is only lost; it must never stop the window.
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}

/// The page's clock is `performance.timeOrigin + performance.now()`, in ms since the Unix epoch,
/// so the app's and the page's points in time compare directly.
fn epoch_ms_of(at: SystemTime) -> f64 {
    at.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

/// The page's marks on the way to first paint, in ms since the Unix epoch.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstPaintMarks {
    navigation_start: f64,
    script_start: f64,
    document_requested: f64,
    document_received: f64,
    document_inserted: f64,
    /// When the frame after the document was inserted finished.
    first_frame: f64,
}

/// A page task or frame that kept the page from answering for over 100 ms.
#[derive(Deserialize)]
pub struct LongTask {
    kind: LongTaskKind,
    /// In ms since the Unix epoch.
    start: f64,
    /// In ms.
    duration: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum LongTaskKind {
    /// One task on the page's main thread.
    Task,
    /// A frame held up by tasks and by style, layout and paint, which a task report misses.
    Frame,
}

impl LongTaskKind {
    fn event(&self) -> &'static str {
        match self {
            Self::Task => "long-task",
            Self::Frame => "long-frame",
        }
    }
}

#[tauri::command]
pub fn report_first_paint(
    window: WebviewWindow,
    marks: FirstPaintMarks,
    timing: State<'_, Timing>,
) {
    timing.first_paint(window.label(), &marks);
}

#[tauri::command]
pub fn report_long_task(window: WebviewWindow, task: LongTask, timing: State<'_, Timing>) {
    timing.long_task(window.label(), &task);
}
