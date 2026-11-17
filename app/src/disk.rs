//! The file system the core's link resolution and local images look at, since the core reads no
//! files: whether a path is a file or a folder, and what a link along it points to.

use std::io;
use std::path::{Path, PathBuf};

use downlight_core::{FileSystem, PathKind};

use crate::platform;

/// This computer's file system.
pub struct Disk;

impl FileSystem for Disk {
    /// Any error other than a missing path, such as an offline share or a folder the reader may
    /// not open, stays an error: the file may well exist, so the reader mustn't be told it doesn't.
    fn kind_of(&self, path: &Path) -> io::Result<Option<PathKind>> {
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => Ok(Some(PathKind::Folder)),
            Ok(_) => Ok(Some(PathKind::File)),
            Err(error) if platform::is_missing(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Looks at the link itself and reads where it points, both without following it, so a link to
    /// a network share connects to nothing (ticket 20). Rust counts a junction as a link too.
    fn link_target(&self, path: &Path) -> io::Result<Option<PathBuf>> {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => std::fs::read_link(path).map(Some),
            Ok(_) => Ok(None),
            Err(error) if platform::is_missing(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own under the temp folder, empty.
    fn temp_folder(test: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("downlight-disk-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn kind_of_tells_files_folders_and_missing_apart() {
        let folder = temp_folder("kinds");
        std::fs::create_dir_all(folder.join("sub")).unwrap();
        std::fs::write(folder.join("notes.txt"), "Notes").unwrap();
        let kinds = [
            Disk.kind_of(&folder.join("notes.txt")),
            Disk.kind_of(&folder.join("sub")),
            Disk.kind_of(&folder.join("missing.md")),
        ];
        // Cleaned up before asserting, so a failing run leaves no folder behind.
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            kinds.map(Result::ok),
            [
                Some(Some(PathKind::File)),
                Some(Some(PathKind::Folder)),
                Some(None)
            ]
        );
    }

    #[test]
    fn missing_path_and_plain_file_are_no_links() {
        let folder = temp_folder("plain");
        std::fs::write(folder.join("logo.png"), "PNG").unwrap();
        let targets = [
            Disk.link_target(&folder.join("logo.png")),
            Disk.link_target(&folder),
            Disk.link_target(&folder.join("missing").join("logo.png")),
        ];
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            targets.map(Result::ok),
            [Some(None), Some(None), Some(None)]
        );
    }

    // Symbolic links to network shares are Windows' (ticket 20). Creating one needs Developer
    // Mode or the right to create symbolic links; without it the tests say so on stderr and pass,
    // since what they check can't be set up.
    #[cfg(windows)]
    mod symlinks {
        use std::os::windows::fs::{symlink_dir, symlink_file};

        use super::*;

        /// A host name that can never resolve (RFC 6761), so even a wrong check would connect to
        /// nothing. Creating a link doesn't look at its target.
        const SHARE: &str = r"\\downlight-test.invalid\share";

        /// Makes `link` with `make`, or says why the test is skipped and gives false.
        fn made(link: &Path, make: impl FnOnce() -> io::Result<()>) -> bool {
            match make() {
                Ok(()) => true,
                Err(error) => {
                    eprintln!(
                        "skipped: can't create the symbolic link {} ({error}); \
                     turn on Developer Mode to run this test",
                        link.display()
                    );
                    false
                }
            }
        }

        /// The image scheme's URL path for `path`, as rendering writes it.
        fn url_path(path: &Path) -> String {
            tauri::Url::from_file_path(path).unwrap().path().to_owned()
        }

        #[test]
        fn image_that_is_a_link_to_a_share_is_no_local_image() {
            let folder = temp_folder("file-link");
            let link = folder.join("logo.png");
            let target = PathBuf::from(SHARE).join("logo.png");
            if made(&link, || symlink_file(&target, &link)) {
                let read = Disk.link_target(&link);
                let image = downlight_core::local_image(&url_path(&link), &Disk);
                std::fs::remove_dir_all(&folder).unwrap();
                assert_eq!(read.ok(), Some(Some(target)));
                assert_eq!(image, None);
            } else {
                std::fs::remove_dir_all(&folder).unwrap();
            }
        }

        #[test]
        fn image_in_a_folder_that_is_a_link_to_a_share_is_no_local_image() {
            let folder = temp_folder("folder-link");
            let link = folder.join("img");
            if made(&link, || symlink_dir(SHARE, &link)) {
                let image = downlight_core::local_image(&url_path(&link.join("logo.png")), &Disk);
                std::fs::remove_dir_all(&folder).unwrap();
                assert_eq!(image, None);
            } else {
                std::fs::remove_dir_all(&folder).unwrap();
            }
        }

        #[test]
        fn image_that_is_a_link_to_a_file_on_this_computer_is_a_local_image() {
            let folder = temp_folder("local-link");
            std::fs::write(folder.join("art.png"), "PNG").unwrap();
            let link = folder.join("logo.png");
            if made(&link, || symlink_file("art.png", &link)) {
                let image = downlight_core::local_image(&url_path(&link), &Disk);
                std::fs::remove_dir_all(&folder).unwrap();
                assert_eq!(image.map(|image| image.path().to_owned()), Some(link));
            } else {
                std::fs::remove_dir_all(&folder).unwrap();
            }
        }
    }
}
