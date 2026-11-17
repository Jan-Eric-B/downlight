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
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, SystemTime};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

use crate::platform;

/// How long the folder stays quiet before a change is reported (the spec's 150 ms).
const QUIET: Duration = Duration::from_millis(150);

/// How often the file is checked whatever the folder's events say. A check is one look at the
/// file's metadata, so a missed save reloads at most this late.
pub const RECHECK: Duration = Duration::from_secs(2);

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

/// Watches one document's file until it is dropped.
pub struct Watch {
    /// Reaches the thread that waits for the folder to go quiet and checks the file. The folder's
    /// watch, which the thread owns, keeps the channel open, so the end is sent.
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
    /// Starts watching `document`'s file, which was `read` when the window last read it (None
    /// while it was missing): a save after that read reports Changed, even one made before the
    /// watch started. `on_change` runs on a thread of the watch's own for each change, until the
    /// watch is dropped. A file missing now reports Back when it appears, and so does one whose
    /// folder is missing now.
    ///
    /// Gives why the folder's events can't be watched, when the folder is there but can't be,
    /// such as on a share or file system without change notifications: its saves are then found
    /// only by the check every [`RECHECK`].
    pub fn start(
        document: &Path,
        read: Option<Stamp>,
        on_change: impl FnMut(Change) + Send + 'static,
    ) -> (Self, Option<notify::Error>) {
        Self::start_checking_every(document, read, RECHECK, on_change)
    }

    /// [`Watch::start`], checking the file `every` so often rather than every [`RECHECK`].
    fn start_checking_every(
        document: &Path,
        read: Option<Stamp>,
        every: Duration,
        on_change: impl FnMut(Change) + Send + 'static,
    ) -> (Self, Option<notify::Error>) {
        let (signals, heard) = mpsc::channel();
        let (folder, unwatched) = FolderWatch::start(document, &signals);
        let watching = Watching {
            document: document.to_owned(),
            stamp: read,
            folder,
            signals: signals.clone(),
            heard,
            every,
            on_change,
        };
        std::thread::spawn(move || watching.run());
        (Self { signals }, unwatched)
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
    /// Starts watching `document`'s folder, or gives why it can't be, unless it is missing.
    fn start(document: &Path, signals: &Sender<Signal>) -> (Self, Option<notify::Error>) {
        let folder = document.parent().unwrap_or(document);
        if !folder.is_dir() {
            return (Self::Missing, None);
        }
        // notify passes the path to CreateFileW as it is, which takes a path over 260 characters
        // only in the `\\?\` form that canonicalizing gives on Windows; the manifest's
        // `longPathAware` would need the system's LongPathsEnabled too. A folder that can't be
        // canonicalized, as on some virtual drives, is watched as named.
        let verbatim = std::fs::canonicalize(folder);
        let folder = verbatim.as_deref().unwrap_or(folder);
        match watch_folder(folder, document, signals) {
            Ok(watcher) => (Self::Watching { _watcher: watcher }, None),
            Err(error) => (Self::Failed, Some(error)),
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
pub struct Stamp {
    size: u64,
    written: Option<SystemTime>,
    created: Option<SystemTime>,
}

impl Stamp {
    /// The stamp of the file at `document` now, None while there is no file there.
    pub fn of(document: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(document).ok().filter(Metadata::is_file)?;
        Some(Self {
            size: metadata.len(),
            written: metadata.modified().ok(),
            created: metadata.created().ok(),
        })
    }
}

/// The watch's thread: what it knows of the file and its folder.
struct Watching<F> {
    document: PathBuf,
    /// The file's when it was last reported, or when the window read it.
    stamp: Option<Stamp>,
    folder: FolderWatch,
    /// For the folder's watch when it is started anew.
    signals: Sender<Signal>,
    heard: Receiver<Signal>,
    /// How often the file is checked.
    every: Duration,
    on_change: F,
}

impl<F: FnMut(Change)> Watching<F> {
    /// Reports each change to the document once the folder has been quiet for [`QUIET`], until
    /// the watch stops. A change that leaves the stamp as it was, such as a virus scanner touching
    /// the file's attributes, reports nothing.
    fn run(mut self) {
        // The first comparison comes as if the file had been touched, since a save between the
        // window's read and the start of the watch gave no event.
        let mut touched = true;
        loop {
            if !touched {
                match self.heard.recv_timeout(self.every) {
                    Ok(Signal::Touched) => {}
                    Ok(Signal::Recheck) | Err(RecvTimeoutError::Timeout) => {
                        if !self.check() {
                            continue;
                        }
                    }
                    Ok(Signal::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            touched = false;
            if !self.quiet() {
                return;
            }
            self.report();
        }
    }

    /// Whether the file differs from its last stamp. If it does, the folder's events missed it,
    /// so the folder is watched anew; a missing folder is watched once it is back.
    fn check(&mut self) -> bool {
        let missed = Stamp::of(&self.document) != self.stamp;
        if missed || matches!(self.folder, FolderWatch::Missing) {
            // The old watch goes only once the new one has started, so no event falls between.
            (self.folder, _) = FolderWatch::start(&self.document, &self.signals);
        }
        missed
    }

    /// Waits until the folder has been quiet for [`QUIET`]; false once the watch stops.
    fn quiet(&self) -> bool {
        loop {
            match self.heard.recv_timeout(QUIET) {
                // The report after the quiet looks at the file anyway.
                Ok(Signal::Touched | Signal::Recheck) => {}
                Err(RecvTimeoutError::Timeout) => return true,
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
            (self.on_change)(change);
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
        watch_since(folder, Stamp::of(&folder.document()))
    }

    /// Watches the folder's document, which was read when it had `read`.
    fn watch_since(folder: &Folder, read: Option<Stamp>) -> (Watch, Receiver<Change>) {
        let (sender, receiver) = mpsc::channel();
        let (watch, unwatched) = Watch::start(&folder.document(), read, move |change| {
            let _ = sender.send(change);
        });
        assert!(unwatched.is_none(), "{unwatched:?}");
        (watch, receiver)
    }

    /// Longer than any test runs, so that only the folder's events or a recheck report a change.
    const NEVER: Duration = Duration::from_secs(3600);

    /// Short, so that a test of what the checks find waits little.
    const OFTEN: Duration = Duration::from_millis(300);

    /// Watches the folder's document as it is now, checking it `every` so often.
    fn watch_checking_every(folder: &Folder, every: Duration) -> (Watch, Receiver<Change>) {
        let (sender, receiver) = mpsc::channel();
        let document = folder.document();
        let (watch, unwatched) =
            Watch::start_checking_every(&document, Stamp::of(&document), every, move |change| {
                let _ = sender.send(change);
            });
        assert!(unwatched.is_none(), "{unwatched:?}");
        (watch, receiver)
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
        let read = Stamp::of(&folder.document());
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
