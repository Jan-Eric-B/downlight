//! Watching a document's file: given its path, it reports Changed, Gone and Back, so the window
//! can reload its document (ticket 10).
//!
//! It watches the file's folder rather than the file, so that an editor that saves by writing a
//! new file and renaming it over the old one is seen too. A save gives several events, so a change
//! is reported once 150 ms have passed without one. What it reports is decided by whether the file
//! exists then, not by the events' kinds, which differ between platforms and editors.
//!
//! The folder's events can stop for good or miss a change without a word (ticket 30's research):
//! notify 8.2.2 on Windows stops watching a folder that was deleted, whose share dropped its
//! session, or that it couldn't watch again, and a burst of events that overflows its buffer is
//! lost. So the file is also checked every [`RECHECK`], and when its window gets the focus
//! ([`Watch::recheck`]). A change a check finds rather than an event means the folder's watch
//! missed it, so the folder is watched anew; a folder that is gone is watched again once it is
//! back.

use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::time::{Duration, SystemTime};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

use crate::platform;

/// How long the folder stays quiet before a change is reported (the spec's 150 ms).
const QUIET: Duration = Duration::from_millis(150);

/// How often the file is checked whatever the folder's events say. A check is one look at the
/// file's metadata, so a missed save reloads at most this late.
const RECHECK: Duration = Duration::from_secs(2);

/// What happened to a watched document's file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The file was saved, in place or by renaming another file over it.
    Changed,
    /// The file was deleted or moved away.
    Gone,
    /// The file is there again after it was gone.
    Back,
}

/// What a watch tells its window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// The document's file changed.
    Change(Change),
    /// The document's folder is there, but its events can't be watched, as on a share or file
    /// system without change notifications: its saves are found only by the check every
    /// `checked_every`, so they reload up to that late.
    EventsUnavailable {
        /// Why, in the system's words.
        reason: String,
        checked_every: Duration,
    },
}

/// Watches one document's file until it is dropped.
pub struct Watch {
    /// Reaches the watch's thread. The thread keeps a sender of its own, for the folder's watch it
    /// starts anew, so dropping this one doesn't end the thread: the end is sent.
    signals: Sender<Signal>,
}

/// What the thread that waits for quiet hears.
enum Signal {
    /// An event for the document's file, or an error that may have hidden one.
    Touched,
    /// The file is to be checked now, whatever the folder's events said.
    Recheck,
    /// The `Watch` was dropped.
    Stop,
}

impl Watch {
    /// Starts watching `document`'s file from `since`, the stamp it had when the window read it: a
    /// save after that read reports Changed, even one made before the watch started. A file
    /// missing now reports Back when it appears, and so does one whose folder is missing now.
    ///
    /// Returns at once: the folder is watched, and `since` waited for, on a thread of the watch's
    /// own, where `on_report` runs for each report until the watch is dropped. A folder that is
    /// there but whose events can't be watched is reported when the watch starts, and when a
    /// missing folder comes back, once rather than at every check that tries again.
    pub fn start(
        document: &Path,
        since: ReadStamp,
        on_report: impl FnMut(Report) + Send + 'static,
    ) -> Self {
        Self::start_checking_every(document, since, RECHECK, on_report)
    }

    /// [`Watch::start`], checking the file `every` so often rather than every [`RECHECK`].
    fn start_checking_every(
        document: &Path,
        since: ReadStamp,
        every: Duration,
        on_report: impl FnMut(Report) + Send + 'static,
    ) -> Self {
        let (signals, heard) = mpsc::channel();
        let watching = Watching {
            document: document.to_owned(),
            // Both are set first thing on the thread, by `Watching::run`.
            stamp: None,
            folder: FolderWatch::Missing,
            events_unavailable: false,
            signals: signals.clone(),
            heard,
            every,
            on_report,
        };
        std::thread::spawn(move || watching.run(since));
        Self { signals }
    }

    /// Has the file checked now, whatever the folder's events said, as when its window gets the
    /// focus: a change the watch missed is then reported at the moment the reader looks.
    pub fn recheck(&self) {
        // The thread ends only after the watch is dropped, so it is there to hear this.
        let _ = self.signals.send(Signal::Recheck);
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // The thread may have ended already; then there is nothing to stop.
        let _ = self.signals.send(Signal::Stop);
    }
}

/// The watch on a document's folder, whose events for the document reach the watch's thread as
/// touches.
enum FolderWatch {
    /// Dropping it stops watching the folder.
    Watching { _watcher: RecommendedWatcher },
    /// The folder isn't there; it is watched once a check finds it back.
    Missing,
    /// The folder is there but couldn't be watched; it is tried again when a check finds a
    /// change.
    Failed,
}

impl FolderWatch {
    /// Starts watching `document`'s folder, unless it is missing; fails when the folder is there
    /// but can't be watched.
    fn start(document: &Path, signals: &Sender<Signal>) -> notify::Result<Self> {
        let folder = document.parent().unwrap_or(document);
        if !folder.is_dir() {
            return Ok(Self::Missing);
        }
        // notify passes the path to CreateFileW as it is, which takes a path over 260 characters
        // only in the `\\?\` form that canonicalizing gives on Windows; the manifest's
        // `longPathAware` would need the system's LongPathsEnabled too. A folder that can't be
        // canonicalized, as on some virtual drives, is watched as named.
        let verbatim = std::fs::canonicalize(folder);
        let folder = verbatim.as_deref().unwrap_or(folder);
        match watch_folder(folder, document, signals) {
            Ok(watcher) => Ok(Self::Watching { _watcher: watcher }),
            // A folder deleted while its watch started is missing, not one whose events can't be
            // watched; the watch looks for it again at each check.
            Err(_) if !folder.is_dir() => Ok(Self::Missing),
            Err(error) => Err(error),
        }
    }
}

/// A watch on `folder` whose events for `document` go to `signals` as touches.
fn watch_folder(
    folder: &Path,
    document: &Path,
    signals: &Sender<Signal>,
) -> notify::Result<RecommendedWatcher> {
    let name = document.file_name().map(|name| folded(Path::new(name)));
    let touched = signals.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let touches_document = event.as_ref().map_or(true, |event| {
            event
                .paths
                .iter()
                .any(|path| path.file_name().map(|n| folded(Path::new(n))) == name)
        });
        if touches_document {
            // The thread ends only after the watch is dropped, and then nothing listens.
            let _ = touched.send(Signal::Touched);
        }
    })?;
    watcher.watch(folder, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

/// A file name as the file system compares names, so `Doc.md` and `doc.md` are one file where
/// the file system ignores case.
fn folded(name: &Path) -> PathBuf {
    platform::fold_case(name.to_owned())
}

/// What tells one version of a file from another without reading it: its size and when it was
/// written and created (a file renamed over it was created later). None while there is no file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    size: u64,
    written: Option<SystemTime>,
    created: Option<SystemTime>,
}

impl Stamp {
    /// The stamp of the file at `document` now, None while there is no file there.
    fn of(document: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(document).ok().filter(Metadata::is_file)?;
        Some(Self {
            size: metadata.len(),
            written: metadata.modified().ok(),
            created: metadata.created().ok(),
        })
    }
}

/// The stamp a document's file had just before the window read it, which a watch starts from
/// (ticket 30): a save after the read then reloads the document, even one made before the watch
/// started. The read notes it, on the render task's thread, with the [`ReadStampNoter`] made with
/// it, and the watch waits for it on its own thread.
pub struct ReadStamp(Receiver<Option<Stamp>>);

/// Notes the [`ReadStamp`] it was made with.
pub struct ReadStampNoter(SyncSender<Option<Stamp>>);

impl ReadStamp {
    /// A stamp still to be noted, with the noter the read notes it with.
    pub fn pending() -> (ReadStampNoter, Self) {
        let (noter, stamp) = mpsc::sync_channel(1);
        (ReadStampNoter(noter), Self(stamp))
    }

    /// The stamp of the file at `document` now, as if the window had just read it.
    #[cfg(test)]
    pub fn now(document: &Path) -> Self {
        let (noter, stamp) = Self::pending();
        noter.note(document);
        stamp
    }

    /// Waits until the stamp has been noted.
    fn noted(self, document: &Path) -> Option<Stamp> {
        // The render task notes the stamp before anything else, so only one that panicked, or never
        // ran because the runtime shut down, drops its noter without noting; the file now is then
        // the nearest there is.
        self.0.recv().unwrap_or_else(|_| Stamp::of(document))
    }
}

impl ReadStampNoter {
    /// Notes the stamp of the file at `document` now, just before it is read.
    pub fn note(self, document: &Path) {
        // Nobody waits for the stamp of a reload, whose watch runs already, nor once the window
        // has left the document.
        let _ = self.0.send(Stamp::of(document));
    }
}

/// The watch's thread: what it knows of the file and its folder.
struct Watching<F> {
    document: PathBuf,
    /// The file's when it was last reported, or when the window read it.
    stamp: Option<Stamp>,
    folder: FolderWatch,
    /// Whether the folder's watch failed to start the last time it was tried, which was reported
    /// then, so that the window tells the reader once rather than after every check.
    events_unavailable: bool,
    /// For the folder's watch when it is started anew.
    signals: Sender<Signal>,
    heard: Receiver<Signal>,
    /// How often the file is checked.
    every: Duration,
    on_report: F,
}

impl<F: FnMut(Report)> Watching<F> {
    /// Watches the folder and reports each change to the document once the folder has been quiet
    /// for [`QUIET`], until the watch stops. A change that leaves the stamp as it was, such as a
    /// virus scanner touching the file's attributes, reports nothing.
    fn run(mut self, since: ReadStamp) {
        self.watch_folder_anew();
        self.stamp = since.noted(&self.document);
        // The first comparison comes without waiting for a change, since a save between the
        // window's read and the start of the watch gave no event.
        loop {
            if !self.quiet() {
                return;
            }
            self.report();
            if !self.wait_for_change() {
                return;
            }
        }
    }

    /// Waits until the folder's events or a check say the file may have changed; false once the
    /// watch stops.
    fn wait_for_change(&mut self) -> bool {
        loop {
            match self.heard.recv_timeout(self.every) {
                Ok(Signal::Touched) => return true,
                Ok(Signal::Recheck) | Err(RecvTimeoutError::Timeout) => {
                    if self.check() {
                        return true;
                    }
                }
                // The thread holds a sender of its own, so the channel can't disconnect; were it
                // to, waiting again would spin.
                Ok(Signal::Stop) | Err(RecvTimeoutError::Disconnected) => return false,
            }
        }
    }

    /// Whether the file differs from its last stamp. If it does, the folder's events missed it,
    /// so the folder is watched anew; a missing folder is watched once it is back.
    fn check(&mut self) -> bool {
        let missed = Stamp::of(&self.document) != self.stamp;
        if missed || matches!(self.folder, FolderWatch::Missing) {
            self.watch_folder_anew();
        }
        missed
    }

    /// Watches the document's folder anew, and reports when its events can't be watched: once in
    /// each run of tries that fail, since each check that finds a change tries again. The old
    /// watch goes only once the new one has started, so no event falls between.
    fn watch_folder_anew(&mut self) {
        self.folder = match FolderWatch::start(&self.document, &self.signals) {
            Ok(folder) => {
                self.events_unavailable = false;
                folder
            }
            Err(error) => {
                if !std::mem::replace(&mut self.events_unavailable, true) {
                    (self.on_report)(Report::EventsUnavailable {
                        reason: error.to_string(),
                        checked_every: self.every,
                    });
                }
                FolderWatch::Failed
            }
        };
    }

    /// Waits until the folder has been quiet for [`QUIET`]; false once the watch stops.
    fn quiet(&self) -> bool {
        loop {
            match self.heard.recv_timeout(QUIET) {
                // The report after the quiet looks at the file anyway.
                Ok(Signal::Touched | Signal::Recheck) => {}
                Err(RecvTimeoutError::Timeout) => return true,
                // As in `wait_for_change`, the channel can't disconnect.
                Ok(Signal::Stop) | Err(RecvTimeoutError::Disconnected) => return false,
            }
        }
    }

    fn report(&mut self) {
        let now = Stamp::of(&self.document);
        let change = match (&self.stamp, &now) {
            (Some(before), Some(after)) => (before != after).then_some(Change::Changed),
            (Some(_), None) => Some(Change::Gone),
            (None, Some(_)) => Some(Change::Back),
            (None, None) => None,
        };
        self.stamp = now;
        if change == Some(Change::Gone)
            && self
                .document
                .parent()
                .is_some_and(|folder| !folder.is_dir())
        {
            // A deleted folder's watch has stopped, and a moved one's follows it elsewhere.
            self.folder = FolderWatch::Missing;
        }
        if let Some(change) = change {
            (self.on_report)(Report::Change(change));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc::{self, Receiver};
    use std::time::Duration;

    use super::*;

    /// Longer than any report takes, so a missing report fails the test rather than hangs it.
    const REPORT_WAIT: Duration = Duration::from_secs(3);

    /// A folder of the test's own with `doc.md` in it, removed when the test ends.
    struct Folder(PathBuf);

    impl Folder {
        fn new(test: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("downlight-watch-{}-{test}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join("doc.md"), "# First").unwrap();
            Self(path)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }

        fn document(&self) -> PathBuf {
            self.file("doc.md")
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Watches the folder's document as it is now; the changes it reports arrive on the receiver.
    fn watch(folder: &Folder) -> (Watch, Receiver<Change>) {
        watch_since(folder, ReadStamp::now(&folder.document()))
    }

    /// Watches the folder's document from `since`, the stamp it had when it was read.
    fn watch_since(folder: &Folder, since: ReadStamp) -> (Watch, Receiver<Change>) {
        watch_changes(folder, since, RECHECK)
    }

    /// Longer than any test runs, so that only the folder's events or a recheck report a change.
    const NEVER: Duration = Duration::from_secs(3600);

    /// Short, so that a test of what the checks find waits little.
    const OFTEN: Duration = Duration::from_millis(300);

    /// Watches the folder's document as it is now, checking it `every` so often.
    fn watch_checking_every(folder: &Folder, every: Duration) -> (Watch, Receiver<Change>) {
        watch_changes(folder, ReadStamp::now(&folder.document()), every)
    }

    /// Watches the folder's document from `since`, checking it `every` so often; what it reports
    /// arrives on the receiver.
    fn watch_reports(
        folder: &Folder,
        since: ReadStamp,
        every: Duration,
    ) -> (Watch, Receiver<Report>) {
        let (sender, receiver) = mpsc::channel();
        let watch = Watch::start_checking_every(&folder.document(), since, every, move |report| {
            let _ = sender.send(report);
        });
        (watch, receiver)
    }

    /// [`watch_reports`] for a folder whose events can be watched, giving only the changes. Any
    /// other report panics the watch's thread, which closes the receiver and so fails the test.
    fn watch_changes(
        folder: &Folder,
        since: ReadStamp,
        every: Duration,
    ) -> (Watch, Receiver<Change>) {
        let (sender, receiver) = mpsc::channel();
        let watch = Watch::start_checking_every(&folder.document(), since, every, move |report| {
            let Report::Change(change) = report else {
                panic!("expected only changes, got {report:?}");
            };
            let _ = sender.send(change);
        });
        (watch, receiver)
    }

    /// Holds `folder` open without sharing it with anyone, so that it is there but notify can't
    /// open it to watch its events, as on a share or file system without change notifications.
    /// Files in it can still be made and written.
    #[cfg(windows)]
    fn held_unwatchable(folder: &Path) -> std::fs::File {
        use std::os::windows::fs::OpenOptionsExt as _;
        // FILE_FLAG_BACKUP_SEMANTICS, without which Windows opens no folder.
        const OPEN_FOLDER: u32 = 0x0200_0000;
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(OPEN_FOLDER)
            .open(folder)
            .unwrap()
    }

    /// Waits out the comparison every watch starts with, so that what comes after is the test's.
    fn started(changes: &Receiver<Change>) {
        assert_eq!(
            changes.recv_timeout(QUIET * 3),
            Err(RecvTimeoutError::Timeout)
        );
    }

    // Story 40: a build that wipes its output folder, or a branch switch.
    #[test]
    fn folder_deleted_and_recreated_reports_gone_then_back_then_saves() {
        let folder = Folder::new("recreated");
        let (_watch, changes) = watch_checking_every(&folder, OFTEN);
        std::fs::remove_dir_all(&folder.0).unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Gone));
        std::fs::create_dir_all(&folder.0).unwrap();
        std::fs::write(folder.document(), "# Built again").unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Back));
        std::fs::write(folder.document(), "# Built once more").unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Changed));
    }

    // notify 8.2.2 stops watching a deleted folder without a word, so the folder's events stop
    // for good unless the watch starts again. Checked only by a recheck here, so the save after
    // it can only be reported by the folder's events.
    #[test]
    fn watch_started_again_once_its_folder_is_back_reports_saves_by_their_events() {
        let folder = Folder::new("restarted");
        let (watch, changes) = watch_checking_every(&folder, NEVER);
        std::fs::remove_dir_all(&folder.0).unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Gone));
        std::fs::create_dir_all(&folder.0).unwrap();
        std::fs::write(folder.document(), "# Built again").unwrap();
        watch.recheck();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Back));
        std::fs::write(folder.document(), "# Built once more").unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Changed));
    }

    // The folder's own events say nothing of the folder being renamed: that is an event of its
    // parent.
    #[test]
    fn folder_renamed_away_and_back_reports_gone_then_back() {
        let folder = Folder::new("renamed");
        let away = folder.0.with_extension("away");
        let _ = std::fs::remove_dir_all(&away);
        let (_watch, changes) = watch_checking_every(&folder, OFTEN);
        std::fs::rename(&folder.0, &away).unwrap();
        let gone = changes.recv_timeout(REPORT_WAIT);
        std::fs::rename(&away, &folder.0).unwrap();
        let back = changes.recv_timeout(REPORT_WAIT);
        assert_eq!((gone, back), (Ok(Change::Gone), Ok(Change::Back)));
    }

    // Downlight's manifest isn't long path aware, and that would need the system's registry too.
    #[test]
    fn folder_over_260_characters_reports_saves_by_their_events() {
        let mut folder = Folder::new("long");
        let deep = (0..3).fold(folder.0.clone(), |path, step| {
            path.join(format!("{step}-{}", "folder".repeat(15)))
        });
        assert!(deep.as_os_str().len() > 260);
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("doc.md"), "# First").unwrap();
        let top = std::mem::replace(&mut folder.0, deep);
        let (_watch, changes) = watch_checking_every(&folder, NEVER);
        std::fs::write(folder.document(), "# Second, written deep down").unwrap();
        let reported = changes.recv_timeout(REPORT_WAIT);
        folder.0 = top;
        assert_eq!(reported, Ok(Change::Changed));
    }

    #[cfg(windows)]
    #[test]
    fn folder_whose_events_cannot_be_watched_reports_so_once_and_saves_by_its_checks() {
        let folder = Folder::new("unwatchable");
        let held = held_unwatchable(&folder.0);
        let (_watch, reports) = watch_reports(&folder, ReadStamp::now(&folder.document()), OFTEN);
        let unavailable = reports.recv_timeout(REPORT_WAIT);
        std::fs::write(folder.document(), "# Second").unwrap();
        let saved = reports.recv_timeout(REPORT_WAIT);
        let after = reports.recv_timeout(OFTEN * 3);
        drop(held);
        assert!(
            matches!(&unavailable, Ok(Report::EventsUnavailable { checked_every, .. }) if *checked_every == OFTEN),
            "{unavailable:?}"
        );
        assert_eq!(saved, Ok(Report::Change(Change::Changed)));
        assert_eq!(after, Err(RecvTimeoutError::Timeout));
    }

    // The owner's decision on ticket 30: a folder that comes back but whose events can't be
    // watched is told as one is when the watch starts, once. Checked only by rechecks, so that the
    // test decides when the folder is looked at.
    #[cfg(windows)]
    #[test]
    fn folder_back_but_its_events_cannot_be_watched_reports_so_once() {
        let folder = Folder::new("back-unwatchable");
        std::fs::remove_dir_all(&folder.0).unwrap();
        let (watch, reports) = watch_reports(&folder, ReadStamp::now(&folder.document()), NEVER);
        let while_missing = reports.recv_timeout(QUIET * 3);
        std::fs::create_dir_all(&folder.0).unwrap();
        let held = held_unwatchable(&folder.0);
        std::fs::write(folder.document(), "# Built again").unwrap();
        watch.recheck();
        let back = [
            reports.recv_timeout(REPORT_WAIT),
            reports.recv_timeout(REPORT_WAIT),
        ];
        std::fs::write(folder.document(), "# Built once more").unwrap();
        watch.recheck();
        let saved = reports.recv_timeout(REPORT_WAIT);
        let after = reports.recv_timeout(QUIET * 3);
        drop(held);
        assert_eq!(while_missing, Err(RecvTimeoutError::Timeout));
        assert!(
            matches!(&back[0], Ok(Report::EventsUnavailable { .. })),
            "{back:?}"
        );
        assert_eq!(back[1], Ok(Report::Change(Change::Back)));
        assert_eq!(saved, Ok(Report::Change(Change::Changed)));
        assert_eq!(after, Err(RecvTimeoutError::Timeout));
    }

    #[test]
    fn recheck_of_an_unchanged_file_reports_nothing() {
        let folder = Folder::new("unchanged");
        let (watch, changes) = watch_checking_every(&folder, NEVER);
        started(&changes);
        watch.recheck();
        assert_eq!(
            changes.recv_timeout(QUIET * 3),
            Err(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn save_between_the_read_and_the_start_of_the_watch_reports_changed() {
        let folder = Folder::new("before-start");
        let read = ReadStamp::now(&folder.document());
        std::fs::write(folder.document(), "# Saved before the watch started").unwrap();
        let (_watch, changes) = watch_since(&folder, read);
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Changed));
    }

    #[test]
    fn rewrite_in_place_reports_changed() {
        let folder = Folder::new("rewrite");
        let (_watch, changes) = watch(&folder);
        std::fs::write(folder.document(), "# Second").unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Changed));
    }

    #[test]
    fn rename_over_the_file_reports_changed() {
        let folder = Folder::new("rename");
        let (_watch, changes) = watch(&folder);
        std::fs::write(folder.file("doc.md.tmp"), "# Second").unwrap();
        std::fs::rename(folder.file("doc.md.tmp"), folder.document()).unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Changed));
        assert_eq!(
            changes.recv_timeout(QUIET * 3),
            Err(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn delete_reports_gone() {
        let folder = Folder::new("delete");
        let (_watch, changes) = watch(&folder);
        std::fs::remove_file(folder.document()).unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Gone));
    }

    #[test]
    fn recreate_after_delete_reports_back() {
        let folder = Folder::new("recreate");
        let (_watch, changes) = watch(&folder);
        std::fs::remove_file(folder.document()).unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Gone));
        std::fs::write(folder.document(), "# Back").unwrap();
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Back));
    }

    #[test]
    fn writes_in_quick_succession_report_one_change() {
        let folder = Folder::new("quick");
        let (_watch, changes) = watch(&folder);
        for text in ["# Second", "# Third", "# Fourth"] {
            std::fs::write(folder.document(), text).unwrap();
            std::thread::sleep(QUIET / 3);
        }
        assert_eq!(changes.recv_timeout(REPORT_WAIT), Ok(Change::Changed));
        assert_eq!(
            changes.recv_timeout(QUIET * 3),
            Err(RecvTimeoutError::Timeout)
        );
    }

    // Windows reports attribute and security changes as modifications too, and a virus scanner or
    // the search indexer makes them without the text changing.
    #[test]
    fn attribute_change_reports_nothing() {
        let folder = Folder::new("attribute");
        let (_watch, changes) = watch(&folder);
        let mut permissions = std::fs::metadata(folder.document()).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(folder.document(), permissions.clone()).unwrap();
        let reported = changes.recv_timeout(QUIET * 3);
        // Writable again, so the folder can be removed.
        #[expect(
            clippy::permissions_set_readonly_false,
            reason = "Windows has no other permissions to give back"
        )]
        permissions.set_readonly(false);
        std::fs::set_permissions(folder.document(), permissions).unwrap();
        assert_eq!(reported, Err(RecvTimeoutError::Timeout));
    }

    #[test]
    fn change_to_another_file_in_the_folder_reports_nothing() {
        let folder = Folder::new("other");
        let (_watch, changes) = watch(&folder);
        std::fs::write(folder.file("other.md"), "# Other").unwrap();
        assert_eq!(
            changes.recv_timeout(QUIET * 3),
            Err(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn dropped_watch_reports_nothing() {
        let folder = Folder::new("dropped");
        let (watch, changes) = watch(&folder);
        drop(watch);
        std::fs::write(folder.document(), "# Second").unwrap();
        assert!(changes.recv_timeout(QUIET * 3).is_err());
    }
}
