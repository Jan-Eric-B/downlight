//! Watching a document's file: given its path, it reports Changed, Gone and Back, so the window
//! can reload its document (ticket 10).
//!
//! It watches the file's folder rather than the file, so that an editor that saves by writing a
//! new file and renaming it over the old one is seen too. A save gives several events, so a change
//! is reported once 150 ms have passed without one. What it reports is decided by whether the file
//! exists then, not by the events' kinds, which differ between platforms and editors.

use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, SystemTime};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

use crate::platform;

/// How long the folder stays quiet before a change is reported (the spec's 150 ms).
const QUIET: Duration = Duration::from_millis(150);

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
    /// Kept for its events; dropping it stops watching the folder.
    _watcher: RecommendedWatcher,
    /// Ends the thread that waits for the folder to go quiet. The watcher's own handler keeps the
    /// channel open, and the watcher stops on a thread of its own, so the end is sent.
    signals: Sender<Signal>,
}

/// What the thread that waits for quiet hears.
enum Signal {
    /// An event for the document's file, or an error that may have hidden one.
    Touched,
    /// The `Watch` was dropped.
    Stop,
}

impl Watch {
    /// Starts watching `document`'s file; `on_change` runs on a thread of the watch's own for each
    /// change, until the watch is dropped. A file missing now reports Back when it appears.
    pub fn start(
        document: &Path,
        on_change: impl FnMut(Change) + Send + 'static,
    ) -> notify::Result<Self> {
        let folder = document.parent().unwrap_or(document).to_owned();
        let name = document.file_name().map(|name| folded(Path::new(name)));
        let (signals, heard) = mpsc::channel();
        let touched = signals.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
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
        watcher.watch(&folder, RecursiveMode::NonRecursive)?;
        let stamp = Stamp::of(document);
        let document = document.to_owned();
        std::thread::spawn(move || report_when_quiet(&document, stamp, &heard, on_change));
        Ok(Self {
            _watcher: watcher,
            signals,
        })
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // The thread may have ended already; then there is nothing to stop.
        let _ = self.signals.send(Signal::Stop);
    }
}

/// A file name as the file system compares names, so `Doc.md` and `doc.md` are one file where
/// the file system ignores case.
fn folded(name: &Path) -> PathBuf {
    platform::fold_case(name.to_owned())
}

/// What tells one version of a file from another without reading it: its size and when it was
/// written and created (a file renamed over it was created later). None while there is no file.
#[derive(PartialEq, Eq)]
struct Stamp {
    size: u64,
    written: Option<SystemTime>,
    created: Option<SystemTime>,
}

impl Stamp {
    fn of(document: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(document).ok().filter(Metadata::is_file)?;
        Some(Self {
            size: metadata.len(),
            written: metadata.modified().ok(),
            created: metadata.created().ok(),
        })
    }
}

/// Reports each change to `document` once the folder has been quiet for [`QUIET`], until the
/// watch stops. `stamp` is the file's when watching started. A change that leaves the stamp as it
/// was, such as a virus scanner touching the file's attributes, reports nothing.
fn report_when_quiet(
    document: &Path,
    mut stamp: Option<Stamp>,
    heard: &Receiver<Signal>,
    mut on_change: impl FnMut(Change),
) {
    loop {
        if !matches!(heard.recv(), Ok(Signal::Touched)) {
            return;
        }
        loop {
            match heard.recv_timeout(QUIET) {
                Ok(Signal::Touched) => {}
                Err(RecvTimeoutError::Timeout) => break,
                Ok(Signal::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        let now = Stamp::of(document);
        let change = match (&stamp, &now) {
            (Some(before), Some(after)) => (before != after).then_some(Change::Changed),
            (Some(_), None) => Some(Change::Gone),
            (None, Some(_)) => Some(Change::Back),
            (None, None) => None,
        };
        stamp = now;
        if let Some(change) = change {
            on_change(change);
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

    /// Watches the folder's document; the changes it reports arrive on the receiver.
    fn watch(folder: &Folder) -> (Watch, Receiver<Change>) {
        let (sender, receiver) = mpsc::channel();
        let watch = Watch::start(&folder.document(), move |change| {
            let _ = sender.send(change);
        })
        .unwrap();
        (watch, receiver)
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
