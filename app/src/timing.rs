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
use std::time::{Duration, Instant, SystemTime};

use serde::Deserialize;
use tauri::{State, WebviewWindow};

use crate::platform;

const SWITCH: &str = "DOWNLIGHT_TIMING";

/// Whether the timing switch is on, and the process start that reports count from.
pub struct Timing {
    /// In ms since the Unix epoch; `None` while the switch is off.
    process_start_ms: Option<f64>,
}

/// The clock for a piece of work in stages, such as reading and rendering a document: when it
/// started, and how long each stage took. [`Timing::report`] reports it as one line.
///
/// It runs whether the switch is on or not, since the work may start before the window's
/// `Timing` is at hand (a render ahead starts when the window is given its document). What the
/// work is reported as is decided when it is reported, since a render started for one reason can
/// be handed over for another (ticket 26).
pub struct Stages {
    started: SystemTime,
    last_lap: Instant,
    laps: Vec<(&'static str, Duration)>,
}

impl Stages {
    /// Starts the clock for a piece of work.
    pub fn start() -> Self {
        Self {
            started: SystemTime::now(),
            last_lap: Instant::now(),
            laps: Vec::new(),
        }
    }

    /// Ends the stage named `stage`, which began when the work or the stage before ended.
    pub fn lap(&mut self, stage: &'static str) {
        let now = Instant::now();
        self.laps.push((stage, now - self.last_lap));
        self.last_lap = now;
    }
}

impl Timing {
    /// Reads the switch from the environment. Call it first thing in `main`: without the OS's
    /// process start time, timing counts from this call.
    pub fn from_env() -> Self {
        let process_start_ms = std::env::var_os(SWITCH)
            .map(|_| epoch_ms_of(platform::process_start().unwrap_or_else(SystemTime::now)));
        Self { process_start_ms }
    }

    /// Whether the switch is on, so a window's page sends its timing reports.
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

    /// Reports `window`'s work in stages as `event`, such as `reload`: when it started, then each
    /// stage's duration.
    pub fn report(&self, window: &str, event: &str, stages: &Stages) {
        if let Some(line) = self.line(window, event, stages) {
            write_line(&line);
        }
    }

    fn line(&self, window: &str, event: &str, stages: &Stages) -> Option<String> {
        let start = self.process_start_ms?;
        let at = ("at", epoch_ms_of(stages.started) - start);
        let laps = stages
            .laps
            .iter()
            .map(|&(stage, duration)| (stage, duration.as_secs_f64() * 1000.0));
        Some(line_of(
            window,
            event,
            &std::iter::once(at).chain(laps).collect::<Vec<_>>(),
        ))
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

    fn reload_shown(&self, window: &str, marks: &ReloadMarks) {
        let Some(start) = self.process_start_ms else {
            return;
        };
        report(
            window,
            "reload-shown",
            &[
                ("at", marks.received - start),
                ("insert", marks.inserted - marks.received),
                ("anchor", marks.anchored - marks.inserted),
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
    write_line(&line_of(window, event, fields));
}

/// `timing <window> <event> key=value ...`, the format `scripts/measure.js` reads.
fn line_of(window: &str, event: &str, fields: &[(&str, f64)]) -> String {
    let mut line = format!("timing {window} {event}");
    for (key, value) in fields {
        line.push_str(&format!(" {key}={value:.1}"));
    }
    line
}

fn write_line(line: &str) {
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

// The page's reports below are serialized only for their contract samples.

/// The page's marks on the way to first paint, in ms since the Unix epoch.
#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
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

/// The page's marks while it shows a reload, in ms since the Unix epoch.
#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct ReloadMarks {
    /// When the reloaded document's HTML arrived.
    received: f64,
    inserted: f64,
    /// When the view was back at the reading position.
    anchored: f64,
}

/// A page task or frame that kept the page from answering for over 100 ms.
#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct LongTask {
    kind: LongTaskKind,
    /// In ms since the Unix epoch.
    start: f64,
    /// In ms.
    duration: f64,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
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

/// A sample of each report the page sends with the timing switch on, by name, which the page's
/// tests check the reports it sends against (`app/src/contract.rs`, ticket 27). A report the app
/// can't take fails silently on the page, so only these samples catch a drift.
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    // A new kind of long task stops the build here: give it a sample below, then name it here.
    let _sampled = |kind: &LongTaskKind| match kind {
        LongTaskKind::Task | LongTaskKind::Frame => {}
    };
    let long = |kind| LongTask {
        kind,
        start: 1250.0,
        duration: 120.0,
    };
    Ok(serde_json::json!({
        "firstPaint": serde_json::to_value(FirstPaintMarks {
            navigation_start: 1000.0,
            script_start: 1040.0,
            document_requested: 1050.0,
            document_received: 1090.0,
            document_inserted: 1100.0,
            first_frame: 1120.0,
        })?,
        "reload": serde_json::to_value(ReloadMarks {
            received: 2000.0,
            inserted: 2030.0,
            anchored: 2040.0,
        })?,
        "longTask": serde_json::to_value(long(LongTaskKind::Task))?,
        "longFrame": serde_json::to_value(long(LongTaskKind::Frame))?,
    }))
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
pub fn report_reload(window: WebviewWindow, marks: ReloadMarks, timing: State<'_, Timing>) {
    timing.reload_shown(window.label(), &marks);
}

#[tauri::command]
pub fn report_long_task(window: WebviewWindow, task: LongTask, timing: State<'_, Timing>) {
    timing.long_task(window.label(), &task);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_line_keeps_the_measure_format() {
        let timing = Timing {
            process_start_ms: Some(1000.0),
        };
        let stages = Stages {
            started: SystemTime::UNIX_EPOCH + Duration::from_millis(1001),
            laps: vec![
                ("read", Duration::from_millis(2)),
                ("render", Duration::from_millis(3)),
            ],
            ..Stages::start()
        };
        assert_eq!(
            timing.line("document-0", "reload", &stages).as_deref(),
            Some("timing document-0 reload at=1.0 read=2.0 render=3.0")
        );
    }

    #[test]
    fn report_with_the_switch_off_is_no_line() {
        let timing = Timing {
            process_start_ms: None,
        };
        assert_eq!(timing.line("document-0", "reload", &Stages::start()), None);
    }

    #[test]
    fn stages_lap_the_time_since_the_lap_before() {
        let mut stages = Stages::start();
        std::thread::sleep(Duration::from_millis(20));
        stages.lap("read");
        stages.lap("render");
        let [(read, first), (render, second)] = stages.laps[..] else {
            panic!("expected two laps, got {:?}", stages.laps);
        };
        assert_eq!((read, render), ("read", "render"));
        assert!(first >= Duration::from_millis(20), "{first:?}");
        assert!(second < first, "{second:?} after {first:?}");
    }
}
