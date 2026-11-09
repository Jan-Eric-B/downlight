use std::path::Path;

/// The extensions of the files Downlight opens as Markdown (spec: Opening documents).
pub const MARKDOWN_EXTENSIONS: [&str; 6] = ["md", "markdown", "mdown", "mkd", "mkdn", "mdwn"];

/// Whether `path` names a Markdown file, by its extension in any case.
pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            MARKDOWN_EXTENSIONS
                .iter()
                .any(|markdown| extension.eq_ignore_ascii_case(markdown))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_markdown_extension_is_markdown() {
        for extension in MARKDOWN_EXTENSIONS {
            let path = format!("notes.{extension}");
            assert!(is_markdown(Path::new(&path)), "{path}");
        }
    }

    #[test]
    fn extension_in_upper_case_is_markdown() {
        assert!(is_markdown(Path::new("README.MD")));
    }

    #[test]
    fn text_file_is_not_markdown() {
        assert!(!is_markdown(Path::new("notes.txt")));
    }

    #[test]
    fn file_without_extension_is_not_markdown() {
        assert!(!is_markdown(Path::new("README")));
    }

    #[test]
    fn markdown_name_with_another_extension_is_not_markdown() {
        assert!(!is_markdown(Path::new("notes.md.txt")));
    }
}
