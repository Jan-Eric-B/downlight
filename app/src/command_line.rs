//! The documents a launch names: `downlight <file>...`, from the shell or from Explorer.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

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
}
