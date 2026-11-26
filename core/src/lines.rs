//! The file's lines, as line numbers count them (glossary: Line number).

/// The lines of `text`, without their line endings, as comrak splits them: at `\n`, `\r\n` and a
/// lone `\r`. A line ending at the end of the text starts no line of its own, so `a\nb` and
/// `a\nb\n` both have two lines and an empty text has none. Everything that reads the file by its
/// line numbers splits it here, so all agree with comrak on where a line ends.
pub(crate) fn lines(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let Some(end) = rest.find(['\r', '\n']) else {
            return Some(std::mem::take(&mut rest));
        };
        let line = &rest[..end];
        let ending = if rest[end..].starts_with("\r\n") {
            2
        } else {
            1
        };
        rest = &rest[end + ending..];
        Some(line)
    })
}
