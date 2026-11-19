use std::path::Path;

use encoding_rs::Encoding;

/// The extensions of Markdown files: those the installer associates with Downlight, Open's dialog
/// shows first and a link shows as a document. Any other text file renders as Markdown too when it
/// is opened from the command line, Open or a drop ([`is_text`]).
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

/// How much of a file's start [`is_text`] looks at: as much as Git looks at to tell a binary file
/// from text, so a caller that hasn't read the whole file reads no more than this.
pub const TEXT_CHECK_LENGTH: usize = 8000;

/// Whether a file whose bytes start with `start` (its first [`TEXT_CHECK_LENGTH`] bytes, or more)
/// is text, which Downlight renders as Markdown whatever its extension (ticket 34). A file is
/// binary when a zero byte comes in its first [`TEXT_CHECK_LENGTH`] bytes, as Git tells it: text
/// in UTF-8 or Windows-1252 has none, while images, programs, archives and Office files have one
/// in their first bytes. A byte order mark makes a file text, since UTF-16 has a zero byte in every
/// ASCII character; UTF-16 without one counts as binary, as decoding couldn't read it anyway.
pub fn is_text(start: &[u8]) -> bool {
    Encoding::for_bom(start).is_some()
        || !start.iter().take(TEXT_CHECK_LENGTH).any(|&byte| byte == 0)
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

    /// The first bytes of a PNG file: its signature, then the length of its first chunk.
    const PNG_START: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR";

    #[test]
    fn file_with_a_zero_byte_near_its_start_is_binary() {
        assert!(!is_text(PNG_START));
        // An `.exe` (`MZ`, then its header) and a `.zip` (or `.docx`) start this way.
        assert!(!is_text(b"MZ\x90\x00\x03\x00"));
        assert!(!is_text(b"PK\x03\x04\x14\x00\x06\x00"));
    }

    #[test]
    fn text_in_any_encoding_the_document_can_have_is_text() {
        assert!(is_text(b"# Notes\r\n\r\nSome text.\r\n"));
        assert!(is_text("Größe: 5 €\n".as_bytes()));
        // Windows-1252, which is no valid UTF-8.
        assert!(is_text(b"Gr\xf6\xdfe: 5 \x80\n"));
        assert!(is_text(b"\xef\xbb\xbf# Notes\n"));
        // UTF-16 LE and BE with their byte order marks: every other byte of ASCII text is zero.
        assert!(is_text(b"\xff\xfe#\x00 \x00N\x00\n\x00"));
        assert!(is_text(b"\xfe\xff\x00#\x00 \x00N\x00\n"));
    }

    #[test]
    fn empty_file_is_text() {
        assert!(is_text(b""));
    }

    #[test]
    fn zero_byte_after_the_start_that_is_looked_at_leaves_a_file_text() {
        let mut bytes = vec![b'a'; TEXT_CHECK_LENGTH];
        bytes.push(0);
        assert!(is_text(&bytes));
        bytes[TEXT_CHECK_LENGTH - 1] = 0;
        assert!(!is_text(&bytes));
    }
}
