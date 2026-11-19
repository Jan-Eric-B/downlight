//! The documents a launch names: `downlight <file>...`, from the shell or from Explorer.

use std::ffi::OsString;
use std::io;
use std::path::{Component, Path, PathBuf, Prefix};

use crate::platform;

/// Takes every argument after the program name as a document's path. A relative path is relative
/// to `launching_folder`, the folder the launch was started in: for a second launch that is its
/// own folder, not the running process's. `..` steps are resolved, so the title names the real
/// folder.
pub fn document_paths(
    args: impl IntoIterator<Item = impl Into<OsString>>,
    launching_folder: &Path,
) -> Vec<PathBuf> {
    args.into_iter()
        .skip(1)
        .map(|arg| {
            let path = launching_folder.join(PathBuf::from(arg.into()));
            // If it can't be resolved, reading the path as it is reports the problem to the
            // reader.
            std::path::absolute(&path).unwrap_or(path)
        })
        .collect()
}

/// The documents' paths to start Downlight again with when `args` name a drive-relative path,
/// such as `C:notes.md`, or None when they name none (ticket 34): every document's path made
/// absolute here, in the launch the reader started.
///
/// Windows keeps a current folder per drive for each process, so only the launching process knows
/// where `C:notes.md` is. A second launch hands its arguments to the running process as it got
/// them, with its current folder alone (tauri-plugin-single-instance offers no way to change
/// them), and the running process would resolve such a path against its own folder on that drive.
/// Started again with whole paths, the launch hands those over instead. A path that can't be made
/// whole gives None, so a launch is never started again with the same arguments.
pub fn relaunch_paths(
    args: impl IntoIterator<Item = impl Into<OsString>>,
    launching_folder: &Path,
) -> Option<Vec<PathBuf>> {
    let args = args.into_iter().map(Into::into).collect::<Vec<OsString>>();
    if !args
        .iter()
        .skip(1)
        .any(|arg| is_drive_relative(Path::new(arg)))
    {
        return None;
    }
    let whole = document_paths(args, launching_folder);
    (!whole.iter().any(|path| is_drive_relative(path))).then_some(whole)
}

/// Starts Downlight again with the documents at `paths`, from [`relaunch_paths`], in this
/// launch's current folder, and returns at once. The new launch may bring its window to the
/// front, as this launch may, which the shell started; as a second launch it hands that on to the
/// running process.
pub fn relaunch(paths: &[PathBuf]) -> io::Result<()> {
    let relaunched = std::process::Command::new(std::env::current_exe()?)
        .args(paths)
        .spawn()?;
    platform::let_take_foreground(relaunched.id());
    Ok(())
}

/// Whether `path` is relative to a drive's current folder: a drive letter without a root, as
/// `C:notes.md`. Only Windows has drive letters.
fn is_drive_relative(path: &Path) -> bool {
    !path.has_root()
        && matches!(
            path.components().next(),
            Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_))
        )
}

// The examples use Windows paths, which only Windows reads as absolute.
#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn relative_path_resolves_against_the_launching_folder() {
        let paths = document_paths(["downlight", r"docs\README.md"], Path::new(r"C:\work"));
        assert_eq!(paths, [PathBuf::from(r"C:\work\docs\README.md")]);
    }

    #[test]
    fn parent_folder_step_is_resolved() {
        let paths = document_paths(["downlight", r"..\a.md"], Path::new(r"C:\work\sub"));
        assert_eq!(paths, [PathBuf::from(r"C:\work\a.md")]);
    }

    #[test]
    fn absolute_path_stays_as_given() {
        let paths = document_paths(["downlight", r"D:\notes\todo.md"], Path::new(r"C:\work"));
        assert_eq!(paths, [PathBuf::from(r"D:\notes\todo.md")]);
    }

    #[test]
    fn every_argument_after_the_program_is_a_document() {
        let paths = document_paths(["downlight", "a.md", "b.md"], Path::new(r"C:\work"));
        assert_eq!(
            paths,
            [
                PathBuf::from(r"C:\work\a.md"),
                PathBuf::from(r"C:\work\b.md")
            ]
        );
    }

    #[test]
    fn launch_without_arguments_names_no_document() {
        let paths = document_paths(["downlight"], Path::new(r"C:\work"));
        assert!(paths.is_empty());
    }

    #[test]
    fn drive_relative_path_is_made_absolute_against_the_launch_s_own_current_folder() {
        // `C:notes.md` with C the drive of this process's current folder, which is then its
        // current folder on that drive too.
        let folder = std::env::current_dir().unwrap();
        let Some(Component::Prefix(drive)) = folder.components().next() else {
            panic!("{} is on no drive", folder.display());
        };
        let mut drive_relative = drive.as_os_str().to_owned();
        drive_relative.push("notes.md");
        let paths = relaunch_paths(["downlight".into(), "a.md".into(), drive_relative], &folder);
        assert_eq!(
            paths,
            Some(vec![folder.join("a.md"), folder.join("notes.md")])
        );
    }

    #[test]
    fn launch_without_a_drive_relative_path_is_handed_over_as_it_is() {
        for path in [
            r"docs\README.md",
            r"\notes.md",
            r"D:\notes\todo.md",
            r"\\server\share\a.md",
        ] {
            assert_eq!(
                relaunch_paths(["downlight", path], Path::new(r"C:\work")),
                None,
                "{path}"
            );
        }
    }
}
