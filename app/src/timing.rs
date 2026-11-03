//! The timing switch. With the environment variable `DOWNLIGHT_TIMING` set, the app reports on
//! stderr how long each window takes to its first paint, the stages on the way there, and every
//! page task or frame longer than 100 ms. It is off by default. Work that grows with a document's
//! size is measured with it (CODING_STANDARDS.md), and `npm run measure` runs it on the test
//! documents.
//!
//! Every report is one line, `timing <window> <event> key=value ...`. Points in time are in ms
//! since the process started; durations are in ms.

use std::collections::HashMap;
use std::io::Write as _;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use tauri::{State, WebviewWindow};

use crate::platform;

const SWITCH: &str = "DOWNLIGHT_TIMING";

/// Whether the timing switch is on, and the clock it reports against.
pub struct Timing {
    /// `None` while the switch is off.
    clock: Option<Clock>,
}

/// The points in time that reports count from: the process start and each window's opening.
struct Clock {
    process_start_ms: f64,
    windows_opened_ms: Mutex<HashMap<String, f64>>,
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
        let clock = std::env::var_os(SWITCH).map(|_| Clock {
            process_start_ms: epoch_ms_of(
                platform::process_start().unwrap_or_else(SystemTime::now),
            ),
            windows_opened_ms: Mutex::default(),
        });
        Self { clock }
    }

    pub fn is_on(&self) -> bool {
        self.clock.is_some()
    }

    /// Notes that a window starts opening; its first paint is also counted from here, which is
    /// what a further window in a running process costs.
    pub fn window_opening(&self, window: &str) {
        let Some(clock) = &self.clock else { return };
        let now = epoch_ms_of(SystemTime::now());
        clock.windows_opened().insert(window.to_owned(), now);
        clock.report(window, "window-opening", &[("at", clock.since_start(now))]);
    }

    pub fn read_and_render(&self, window: &str, times: &ReadAndRenderTimes) {
        let Some(clock) = &self.clock else { return };
        clock.report(
            window,
            "read-and-render",
            &[
                ("at", clock.since_start(epoch_ms_of(times.started))),
                ("read", times.read.as_secs_f64() * 1000.0),
                ("render", times.render.as_secs_f64() * 1000.0),
            ],
        );
    }

    fn first_paint(&self, window: &str, marks: &FirstPaintMarks) {
        let Some(clock) = &self.clock else { return };
        let opened = clock
            .windows_opened()
            .get(window)
            .copied()
            .unwrap_or(clock.process_start_ms);
        clock.report(
            window,
            "first-paint",
            &[
                ("at", clock.since_start(marks.first_frame)),
                ("since_window", marks.first_frame - opened),
                ("navigation", clock.since_start(marks.navigation_start)),
                ("script", clock.since_start(marks.script_start)),
                ("requested", clock.since_start(marks.document_requested)),
                ("received", clock.since_start(marks.document_received)),
                ("inserted", clock.since_start(marks.document_inserted)),
            ],
        );
    }

    fn long_task(&self, window: &str, task: &LongTask) {
        let Some(clock) = &self.clock else { return };
        clock.report(
            window,
            task.kind.event(),
            &[
                ("at", clock.since_start(task.start)),
                ("duration", task.duration),
            ],
        );
    }
}

impl Clock {
    fn since_start(&self, epoch_ms: f64) -> f64 {
        epoch_ms - self.process_start_ms
    }

    fn windows_opened(&self) -> std::sync::MutexGuard<'_, HashMap<String, f64>> {
        // The map stays valid even if a thread panicked while holding the lock.
        self.windows_opened_ms
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn report(&self, window: &str, event: &str, fields: &[(&str, f64)]) {
        let mut line = format!("timing {window} {event}");
        for (key, value) in fields {
            line.push_str(&format!(" {key}={value:.1}"));
        }
        self.write(&line);
    }

    fn write(&self, line: &str) {
        // A report that can't be written is only lost; it must never stop the window.
        let _ = writeln!(std::io::stderr().lock(), "{line}");
    }
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
