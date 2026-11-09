use std::borrow::Cow;

use encoding_rs::{Encoding, WINDOWS_1252};

/// Turns a document's file into text, by the spec's only loading rules: a byte order mark decides
/// the encoding (UTF-8, UTF-16 LE or UTF-16 BE) and is left out of the text; without one, valid
/// UTF-8 is UTF-8, and anything else is Windows-1252, which gives every byte a character.
pub(crate) fn decode(bytes: &[u8]) -> Cow<'_, str> {
    if let Some((encoding, mark_length)) = Encoding::for_bom(bytes) {
        let (text, _) = encoding.decode_without_bom_handling(&bytes[mark_length..]);
        return text;
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Cow::Borrowed(text),
        Err(_) => WINDOWS_1252.decode_without_bom_handling(bytes).0,
    }
}
