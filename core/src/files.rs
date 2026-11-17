//! The file system as the core sees it: through the caller's lookups, since the core reads no
//! files. Following a link or serving a local image looks at a path only once the core knows where
//! it leads. Looking at anything on a network share connects to the computer that shares it, and
//! Windows may send that computer the reader's sign-in (user story 28). A symbolic link on this
//! computer can lead to a share too, since Windows follows it when the file is opened, so each
//! link along a path is read without opening what it points to (ticket 20).

use std::io;
use std::path::{Component, Path, PathBuf, Prefix};
use std::time::SystemTime;

/// What a path names on the disk, as the caller's lookup tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    File,
    Folder,
}

/// The caller's lookups in the file system, which link resolution and local images ask.
pub trait FileSystem {
    /// Whether `path` is a file or a folder, None when nothing is there, or an error when that
    /// can't be told. It follows links, and connects to a network share the path is on, so it
    /// is asked only about a path that stays on this computer or one the reader agreed to.
    fn kind_of(&self, path: &Path) -> io::Result<Option<PathKind>>;

    /// The target of the symbolic link or junction at `path`, as the link writes it, or None when
    /// `path` is something else or nothing. It must never open the target: the target may be on a
    /// network share. Only asked about paths on this computer.
    fn link_target(&self, path: &Path) -> io::Result<Option<PathBuf>>;

    /// When the file at `path` was last saved, None when nothing is there, or an error when that
    /// can't be told. Like [`kind_of`](Self::kind_of) it follows links, so it is asked only about
    /// a path that stays on this computer.
    fn modified(&self, path: &Path) -> io::Result<Option<SystemTime>>;
}

/// Where a path leads once Windows has followed the links along it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Leads {
    /// To a file or folder on this computer, or to nothing there.
    ThisComputer,
    /// To a network share on the computer `host`, which looking at the path connects to.
    Share { host: String },
    /// Anywhere else, such as a device or another volume named by its GUID, or through more links
    /// than Windows follows. Never looked at.
    Elsewhere,
}

/// The most links Windows follows in one path.
const MOST_LINKS: usize = 63;

/// Where `path`, a whole path, leads. Asks `files` for the target of each link along a path on
/// this computer, from its root down, so a link is read only once the folders it is in are known
/// to stay here; a share's own paths are never looked at.
pub(crate) fn leads(path: &Path, files: &impl FileSystem) -> io::Result<Leads> {
    let mut path = path.to_owned();
    for _ in 0..=MOST_LINKS {
        if let Some(host) = share_host(&path) {
            return Ok(Leads::Share { host });
        }
        if !is_local(&path) {
            return Ok(Leads::Elsewhere);
        }
        match through_first_link(&path, files)? {
            Some(next) => path = next,
            None => return Ok(Leads::ThisComputer),
        }
    }
    Ok(Leads::Elsewhere)
}

/// `path` with its first link replaced by where the link points, or None when no part of it is a
/// link.
fn through_first_link(path: &Path, files: &impl FileSystem) -> io::Result<Option<PathBuf>> {
    let mut walked = PathBuf::new();
    let mut rest = path.components();
    while let Some(component) = rest.next() {
        walked.push(component);
        if !matches!(component, Component::Normal(_)) {
            continue;
        }
        let Some(target) = files.link_target(&walked)? else {
            continue;
        };
        // A relative target is relative to the link's folder; an absolute one replaces it.
        let mut next = walked.parent().unwrap_or(&walked).join(target);
        if !rest.as_path().as_os_str().is_empty() {
            next.push(rest.as_path());
        }
        return Ok(Some(without_dots(next)));
    }
    Ok(None)
}

/// `path` with its `.` and `..` resolved by the names, as Windows resolves them in a link's
/// target, never by looking at the folders. A path that isn't whole, such as `C:pics` (relative to
/// drive C's current folder), stays as it is, so it leads elsewhere: where it leads depends on the
/// process's current folders, which the core doesn't use (`std::path::absolute` would).
fn without_dots(path: PathBuf) -> PathBuf {
    if !path.is_absolute() {
        return path;
    }
    let mut resolved = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            // `..` at the root stays at the root.
            Component::ParentDir => {
                if matches!(
                    resolved.components().next_back(),
                    Some(Component::Normal(_))
                ) {
                    resolved.pop();
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                resolved.push(component);
            }
        }
    }
    resolved
}

/// The computer whose network share `path` is on, from `\\host\share` or `\\?\UNC\host\share`.
fn share_host(path: &Path) -> Option<String> {
    match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::UNC(host, _) | Prefix::VerbatimUNC(host, _) => {
                Some(host.to_string_lossy().into_owned())
            }
            _ => None,
        },
        _ => None,
    }
}

/// Whether `path` names a network share on the computer `host`, by its name alone: `\\host\share`
/// and `\\?\UNC\host\share` are the same, and a `file://host/` URL gives the first. Windows
/// ignores the case of a computer's name, but only ASCII letters are compared without it, so a
/// name that differs in any other way, such as another spelling of the same computer (its full
/// DNS name or its address), counts as another computer.
pub(crate) fn is_on_share_of(path: &Path, host: &str) -> bool {
    share_host(path).is_some_and(|own| own.eq_ignore_ascii_case(host))
}

/// Whether `path` is a whole path on this computer by its name alone: on Windows, one on a drive,
/// never a network share or a device.
pub(crate) fn is_local(path: &Path) -> bool {
    path.is_absolute()
        && match path.components().next() {
            Some(Component::Prefix(prefix)) => {
                matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
            }
            // Only Windows has prefixes.
            _ => true,
        }
}

/// A file system for the tests: every path is a file, except folders, which end in a slash or in
/// `folder.md`, and missing paths, which contain `missing`. It has the links named here, a link
/// under a folder named `unreadable` can't be read, and nothing on the server `offline` can be
/// reached. Only the files named by `modified_at` tell when they were saved. Every path it is
/// asked about is kept, to check what was looked at.
#[cfg(test)]
pub(crate) mod fake {
    use std::collections::BTreeMap;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::SystemTime;

    use super::{FileSystem, PathKind};

    #[derive(Default)]
    pub(crate) struct FakeFiles {
        links: BTreeMap<PathBuf, PathBuf>,
        modified: BTreeMap<PathBuf, SystemTime>,
        pub(crate) asked: Mutex<Vec<PathBuf>>,
    }

    impl FakeFiles {
        /// `link` is a link to `target`, as the link writes it.
        pub(crate) fn with_link(mut self, link: &str, target: &str) -> Self {
            self.links
                .insert(PathBuf::from(link), PathBuf::from(target));
            self
        }

        /// The file at `path` was last saved at `time`.
        pub(crate) fn modified_at(mut self, path: &str, time: SystemTime) -> Self {
            self.modified.insert(PathBuf::from(path), time);
            self
        }

        fn ask(&self, path: &Path) {
            self.asked.lock().unwrap().push(path.to_owned());
        }

        /// Whether any lookup touched a path on a network share.
        pub(crate) fn touched_a_share(&self) -> bool {
            self.asked
                .lock()
                .unwrap()
                .iter()
                .any(|path| path.to_string_lossy().starts_with(r"\\"))
        }
    }

    impl FileSystem for FakeFiles {
        fn kind_of(&self, path: &Path) -> io::Result<Option<PathKind>> {
            self.ask(path);
            let text = path.to_string_lossy();
            if text.starts_with(r"\\offline\") {
                Err(io::ErrorKind::PermissionDenied.into())
            } else if text.contains("missing") {
                Ok(None)
            } else if text.ends_with('\\') || text.ends_with("folder.md") {
                Ok(Some(PathKind::Folder))
            } else {
                Ok(Some(PathKind::File))
            }
        }

        fn link_target(&self, path: &Path) -> io::Result<Option<PathBuf>> {
            self.ask(path);
            if path.to_string_lossy().contains("unreadable") {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            Ok(self.links.get(path).cloned())
        }

        fn modified(&self, path: &Path) -> io::Result<Option<SystemTime>> {
            self.ask(path);
            Ok(self.modified.get(path).copied())
        }
    }
}

// Windows paths: v1 ships for Windows only, and drive letters and network shares are Windows'.
#[cfg(all(test, windows))]
mod tests {
    use std::path::Path;

    use super::fake::FakeFiles;
    use super::*;

    #[expect(
        clippy::panic,
        reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
    )]
    fn leads_from(path: &str, files: &FakeFiles) -> Leads {
        leads(Path::new(path), files).unwrap_or_else(|error| panic!("{path}: {error:?}"))
    }

    fn share(host: &str) -> Leads {
        Leads::Share {
            host: host.to_owned(),
        }
    }

    #[test]
    fn path_on_a_drive_without_links_stays_on_this_computer() {
        let files = FakeFiles::default();
        assert_eq!(
            leads_from(r"C:\docs\img\logo.png", &files),
            Leads::ThisComputer
        );
        assert_eq!(
            *files.asked.lock().unwrap(),
            [r"C:\docs", r"C:\docs\img", r"C:\docs\img\logo.png"].map(PathBuf::from)
        );
    }

    #[test]
    fn path_on_a_share_leads_to_its_host_without_a_lookup() {
        for path in [
            r"\\server\share\logo.png",
            r"\\?\UNC\server\share\logo.png",
            r"\\server\share",
        ] {
            let files = FakeFiles::default();
            assert_eq!(leads_from(path, &files), share("server"), "{path}");
            assert!(files.asked.lock().unwrap().is_empty(), "{path}");
        }
    }

    #[test]
    fn link_to_a_share_leads_to_the_share_without_looking_at_it() {
        for (link, target, path) in [
            (
                r"C:\docs\logo.png",
                r"\\server\share\x.png",
                r"C:\docs\logo.png",
            ),
            (
                r"C:\docs\img",
                r"\\server\share\pics",
                r"C:\docs\img\logo.png",
            ),
            (
                r"C:\docs\img",
                r"\\?\UNC\server\share\pics",
                r"C:\docs\img\logo.png",
            ),
        ] {
            let files = FakeFiles::default().with_link(link, target);
            assert_eq!(leads_from(path, &files), share("server"), "{link}");
            assert!(!files.touched_a_share(), "{link}: {:?}", files.asked);
        }
    }

    #[test]
    fn link_to_this_computer_is_followed_to_where_it_leads() {
        for target in [r"C:\pics", r"\\?\C:\pics", r"..\pics", r"\pics"] {
            let files = FakeFiles::default().with_link(r"C:\docs\img", target);
            assert_eq!(
                leads_from(r"C:\docs\img\logo.png", &files),
                Leads::ThisComputer,
                "{target}"
            );
        }
    }

    #[test]
    fn link_to_a_drive_without_a_root_leads_elsewhere() {
        // `C:pics` is relative to drive C's current folder, which only the process knows, so the
        // core can't tell where it leads.
        for target in ["C:pics", r"D:..\pics", "C:"] {
            let files = FakeFiles::default().with_link(r"C:\docs\img", target);
            assert_eq!(
                leads_from(r"C:\docs\img\logo.png", &files),
                Leads::Elsewhere,
                "{target}"
            );
        }
    }

    #[test]
    fn link_with_dots_is_followed_by_the_names_alone() {
        let files = FakeFiles::default()
            .with_link(r"C:\docs\img", r"..\.\..\..\shared\pics")
            .with_link(r"C:\shared", r"\\nas\media");
        assert_eq!(leads_from(r"C:\docs\img\logo.png", &files), share("nas"));
        assert!(!files.touched_a_share(), "{:?}", files.asked);
    }

    #[test]
    fn chain_of_links_that_ends_on_a_share_leads_to_the_share() {
        let files = FakeFiles::default()
            .with_link(r"C:\docs\img", r"D:\shared")
            .with_link(r"D:\shared", r"..\mounted\pics")
            .with_link(r"D:\mounted", r"\\nas\media");
        assert_eq!(leads_from(r"C:\docs\img\logo.png", &files), share("nas"));
        assert!(!files.touched_a_share(), "{:?}", files.asked);
    }

    #[test]
    fn link_to_a_device_or_a_volume_leads_elsewhere() {
        for target in [
            r"\\?\Volume{0b1e3f6c-0000-0000-0000-100000000000}\pics",
            r"\\.\pipe\pics",
        ] {
            let files = FakeFiles::default().with_link(r"C:\docs\img", target);
            assert_eq!(
                leads_from(r"C:\docs\img\logo.png", &files),
                Leads::Elsewhere,
                "{target}"
            );
        }
    }

    #[test]
    fn link_loop_leads_elsewhere() {
        let files = FakeFiles::default()
            .with_link(r"C:\a", r"C:\b")
            .with_link(r"C:\b", r"C:\a");
        assert_eq!(leads_from(r"C:\a\logo.png", &files), Leads::Elsewhere);
    }

    #[test]
    fn link_that_cannot_be_read_is_an_error() {
        let files = FakeFiles::default();
        assert!(leads(Path::new(r"C:\unreadable\logo.png"), &files).is_err());
    }
}
