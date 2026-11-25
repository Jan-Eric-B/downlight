//! Deleting pasted files (paste-markdown ticket 02, ADR 0006): a pasted file is temporary, so it
//! is deleted once no window shows it, has it on its way or holds it in its history. The table
//! decides: each change that lets go of documents hands them to [`Documents::let_go`], after the
//! table's lock, and a thread of its own deletes those that are pasted files no window holds any
//! more, off the main thread, which closes windows. It checks and deletes each under the table's
//! lock, so that a window given the file meanwhile keeps it.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock, Weak};
use std::time::{Duration, SystemTime};

use super::{Documents, Inner, document_key};
use crate::paste::PastedFolder;
use crate::platform;

/// The pasted folder, and the thread that deletes from it, started by the first file let go of.
pub(super) struct Pastes {
    folder: PastedFolder,
    deleting: OnceLock<Sender<Job>>,
    /// What runs the next time a pasted file is deleted, just before it is
    /// ([`Documents::before_next_delete`]).
    #[cfg(test)]
    before_next_delete: std::sync::Mutex<Option<BeforeDelete>>,
}

/// What another thread does just before a pasted file is deleted.
#[cfg(test)]
type BeforeDelete = Box<dyn FnOnce() + Send>;

impl Pastes {
    pub(super) const fn new(folder: PastedFolder) -> Self {
        Self {
            folder,
            deleting: OnceLock::new(),
            #[cfg(test)]
            before_next_delete: std::sync::Mutex::new(None),
        }
    }

    /// Runs what [`Documents::before_next_delete`] was given, if it was.
    #[cfg(test)]
    fn before_delete(&self) {
        // Taken out of its lock before it runs, since what it does may delete a file too.
        let meanwhile = self
            .before_next_delete
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(meanwhile) = meanwhile {
            meanwhile();
        }
    }
}

/// A document a window holds, borrowed from where the table keeps it: named fields rather than a
/// pair of paths, so that the two can't be swapped.
#[derive(Debug, Clone, Copy)]
pub(super) struct HeldDocument<'a> {
    /// Tells whether a window still holds it and whether it is pasted.
    pub(super) key: &'a Path,
    /// As it was given, which it is deleted by.
    pub(super) path: &'a Path,
}

/// What holds documents a window shows, has on its way or can return to: a window's record, what
/// it was, its history, and what the table took out of them.
pub(super) trait HoldsDocuments {
    /// Every document this holds.
    fn held(&self) -> impl Iterator<Item = HeldDocument<'_>>;
}

impl HoldsDocuments for HeldDocument<'_> {
    fn held(&self) -> impl Iterator<Item = HeldDocument<'_>> {
        std::iter::once(*self)
    }
}

impl<T: HoldsDocuments> HoldsDocuments for Option<T> {
    fn held(&self) -> impl Iterator<Item = HeldDocument<'_>> {
        self.iter().flat_map(|holds| holds.held())
    }
}

impl<T: HoldsDocuments> HoldsDocuments for Box<T> {
    fn held(&self) -> impl Iterator<Item = HeldDocument<'_>> {
        (**self).held()
    }
}

impl<T: HoldsDocuments> HoldsDocuments for Vec<T> {
    fn held(&self) -> impl Iterator<Item = HeldDocument<'_>> {
        self.iter().flat_map(|holds| holds.held())
    }
}

impl HoldsDocuments for super::WindowRecord {
    /// The one it shows, the one on its way, and those in its history, also what it was before
    /// the one on its way.
    fn held(&self) -> impl Iterator<Item = HeldDocument<'_>> {
        [Some(&self.now), self.before_arrival.as_ref()]
            .into_iter()
            .flatten()
            .flat_map(HoldsDocuments::held)
    }
}

impl HoldsDocuments for super::WindowState {
    /// The one shown, and those in the history.
    fn held(&self) -> impl Iterator<Item = HeldDocument<'_>> {
        self.shown
            .iter()
            .map(|shown| HeldDocument {
                key: &shown.key,
                path: &shown.path,
            })
            .chain(self.history.held())
    }
}

/// A document the table let go of, owned for the deleting thread.
struct ReleasedDocument {
    key: PathBuf,
    path: PathBuf,
}

impl From<HeldDocument<'_>> for ReleasedDocument {
    fn from(held: HeldDocument<'_>) -> Self {
        Self {
            key: held.key.to_owned(),
            path: held.path.to_owned(),
        }
    }
}

/// What the deleting thread does, in the order it is asked.
enum Job {
    /// Deletes each of these that is a pasted file no window holds.
    Delete(Vec<ReleasedDocument>),
    /// Deletes every file in the pasted folder last modified before `started` that no window
    /// holds.
    Sweep { started: SystemTime },
    /// Says on the sender that every job before it is done.
    Finished(Sender<()>),
}

/// How long [`Documents::finish_deleting_pastes`] waits at most: deleting a few files takes
/// milliseconds, and what is left after this goes at the next startup's sweep.
const FINISH_WAIT: Duration = Duration::from_secs(2);

impl Documents {
    /// The folder pastes are written to.
    pub fn pasted_folder(&self) -> &PastedFolder {
        &self.inner.pastes.folder
    }

    /// Waits until every pasted file let go of so far is deleted, or [`FINISH_WAIT`] has passed:
    /// as the process ends, which would otherwise leave the file of the last window closed.
    pub fn finish_deleting_pastes(&self) {
        let Some(deleting) = self.inner.pastes.deleting.get() else {
            return;
        };
        let (finished, wait) = mpsc::channel();
        if deleting.send(Job::Finished(finished)).is_ok() {
            // Not finished in time, the files are left for the next startup's sweep.
            let _ = wait.recv_timeout(FINISH_WAIT);
        }
    }

    /// Empties the pasted folder of what a crash or a failed delete left, on the deleting
    /// thread, so that the caller, starting the first window's show, never waits for it. Only
    /// when the process starts: a second launch hands over to the running process before this.
    /// A file a window holds by then stays, as a leftover paste opened from the file manager
    /// that started Downlight, and goes as any pasted file does.
    ///
    /// Only files last modified before this process started go: one written since is a paste of
    /// another Downlight process still running, such as one that hadn't quite ended when this
    /// one started, whose window may still show it. Where the start isn't known, the sweep
    /// counts from now.
    pub fn sweep_pastes(&self) {
        let started = platform::process_start().unwrap_or_else(SystemTime::now);
        self.deleting(Job::Sweep { started });
    }

    /// The next time a pasted file is deleted, `meanwhile` runs first on the deleting thread, as
    /// what another thread does just then. So a test puts a window's request for the file at the
    /// moment of its choosing, as [`Recorder::before_next_title`] puts a show into another.
    ///
    /// [`Recorder::before_next_title`]: super::recorder::Recorder::before_next_title
    #[cfg(test)]
    pub(super) fn before_next_delete(&self, meanwhile: impl FnOnce() + Send + 'static) {
        *self
            .inner
            .pastes
            .before_next_delete
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(meanwhile));
    }

    /// The table let go of `left`, which is dropped here: the caller has released the table's
    /// lock, since dropping a watch stops it. Each pasted file it held that no window holds any
    /// more is deleted, off the calling thread. Dropping and letting go in one call, so that no
    /// change can do the one and forget the other.
    pub(super) fn let_go(&self, left: impl HoldsDocuments) {
        let let_go: Vec<ReleasedDocument> = left.held().map(ReleasedDocument::from).collect();
        if !let_go.is_empty() {
            self.deleting(Job::Delete(let_go));
        }
        drop(left);
    }

    fn deleting(&self, job: Job) {
        let deleting = self
            .inner
            .pastes
            .deleting
            .get_or_init(|| start_deleting(Arc::downgrade(&self.inner)));
        // A thread that couldn't start leaves the files for the next startup's sweep.
        let _ = deleting.send(job);
    }
}

/// Starts the thread that deletes pasted files for the documents module `inner`, and gives what
/// asks it. It ends with the module, which holds the only sender.
fn start_deleting(inner: Weak<Inner>) -> Sender<Job> {
    let (sender, jobs) = mpsc::channel::<Job>();
    // A thread that can't start drops the receiver, so every job sent is refused, and the files
    // are left for the next startup's sweep.
    let _ = std::thread::Builder::new()
        .name("pasted files".to_owned())
        .spawn(move || {
            for job in jobs {
                match job {
                    Job::Delete(let_go) => delete_unheld(&inner, let_go),
                    Job::Sweep { started } => sweep(&inner, started),
                    // The waiter may have given up already.
                    Job::Finished(finished) => drop(finished.send(())),
                }
            }
        });
    sender
}

/// Deletes every file in `inner`'s pasted folder last modified before `started` that no window
/// holds.
fn sweep(inner: &Weak<Inner>, started: SystemTime) {
    let Some(files) = inner
        .upgrade()
        .map(|inner| files_modified_before(inner.pastes.folder.path(), started))
    else {
        return;
    };
    let files = files
        .into_iter()
        .map(|path| ReleasedDocument {
            key: document_key(&path),
            path,
        })
        .collect();
    delete_unheld(inner, files);
}

/// Every file in `folder` last modified before `started`; none while the folder doesn't exist or
/// can't be read. A file whose modified time can't be read stays, as one that may be in use.
fn files_modified_before(folder: &Path, started: SystemTime) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.metadata().is_ok_and(|file| {
                file.is_file() && file.modified().is_ok_and(|modified| modified < started)
            })
        })
        .map(|entry| entry.path())
        .collect()
}

/// Deletes each of `let_go` that is a pasted file no window of `inner`'s table holds. A pasted
/// file is one in the pasted folder, compared as the table compares documents, so that a pasted
/// file named by another path, as through a short name or a link, still counts.
fn delete_unheld(inner: &Weak<Inner>, let_go: Vec<ReleasedDocument>) {
    let Some(inner) = inner.upgrade() else {
        return;
    };
    let documents = Documents { inner };
    // Once per job rather than per file, since it resolves the folder on the file system.
    let folder = document_key(documents.pasted_folder().path());
    let pasted: Vec<ReleasedDocument> = let_go
        .into_iter()
        .filter(|document| document.key.parent() == Some(folder.as_path()))
        .collect();
    for document in pasted {
        // Checked and deleted under one hold of the table's lock, which every claim of a document
        // takes, so that no window can be given the file between the two and lose it (ticket
        // 02's review). One file at a time, so that the main thread, which takes the lock as a
        // window closes, waits for at most one delete of a file in the temporary folder, a
        // fraction of a millisecond, even during a sweep of many.
        let table = documents.lock();
        if table.holds(&document.key) {
            continue;
        }
        #[cfg(test)]
        documents.inner.pastes.before_delete();
        // A file already gone is as good as deleted. Any other failure, such as a program that
        // holds the file open, leaves it for the next startup's sweep: the reader can't act on
        // it, so nobody is told.
        let _ = std::fs::remove_file(document.path);
        drop(table);
    }
}

/// The documents a window holds, as the deleting of pasted files asks.
impl super::WindowTable {
    /// Whether any window shows the document with `key`, has it on its way or holds it in its
    /// history.
    fn holds(&self, key: &Path) -> bool {
        self.windows
            .values()
            .flat_map(HoldsDocuments::held)
            .any(|held| held.key == key)
    }
}
