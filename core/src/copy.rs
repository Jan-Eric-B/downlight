//! Copy as Markdown (copy-as-markdown tickets 01 to 03): the document's Markdown for the pieces
//! of its file the reader's selection covers, as the page names them by the line numbers its
//! rendered blocks carry. The core reads no file: the app passes the bytes the render it shows
//! took.

use std::fmt;

use crate::decode::decode;
use crate::lines::lines;

/// A piece of a document's file to copy. A range of whole lines copies a run of blocks, and also
/// the part of a code block a selection touches (ticket 02): its opening fence line, the code
/// lines touched and its closing fence line are pieces of their own, and so are a table's header
/// row, its delimiter row and the rows a selection touches (ticket 03). A span copies a selection
/// within one table cell.
///
/// Serialized as the page names a piece to the app (`{ "piece": "lines", "first": 3, "last": 5 }`),
/// so the app takes the page's request as pieces without a type of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "piece", rename_all = "camelCase")]
pub enum Piece {
    /// The lines from `first` to `last`, both included, 1-based as the rendered document's line
    /// numbers count them.
    Lines { first: usize, last: usize },
    /// The columns from `first` to `last` of the line `line`, both included, as a table cell's
    /// line numbers (`data-sourcepos`) count them: 1-based, in bytes of the line's UTF-8 text as
    /// the render decoded it.
    Span {
        line: usize,
        first: usize,
        last: usize,
    },
}

/// Why the Markdown of the pieces can't be copied. Only a stale or broken request names a piece
/// the file doesn't have: the page takes every piece from the render the app keeps the bytes of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyError {
    /// The piece names a line the file doesn't have, or a column its line doesn't have, or ends
    /// before it starts.
    NotInTheFile(Piece),
    /// The span from column `first` to `last` of the line `line` starts or ends inside a
    /// character of its line.
    SplitsACharacter {
        line: usize,
        first: usize,
        last: usize,
    },
}

impl fmt::Display for CopyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInTheFile(Piece::Lines { first, last }) => {
                write!(f, "lines {first} to {last} aren't in the file")
            }
            Self::NotInTheFile(Piece::Span { line, first, last }) => {
                write!(
                    f,
                    "columns {first} to {last} of line {line} aren't in the file"
                )
            }
            Self::SplitsACharacter { line, first, last } => {
                write!(
                    f,
                    "columns {first} to {last} of line {line} split a character"
                )
            }
        }
    }
}

impl std::error::Error for CopyError {}

/// The Markdown of `pieces` of the document whose file holds `bytes`, or of the whole file for no
/// pieces, in the order given. The bytes are decoded as [`render`](crate::render) decodes them,
/// and lines end where they end for the render: at `\n`, `\r\n` or a lone `\r`.
///
/// The whole file copies as it is written. Of pieces, the whitespace every copied line that isn't
/// blank starts with is dropped, so a nested list item pastes as a list item rather than as an
/// indented code block, and its inner items keep their indentation under it; every other prefix
/// stays, such as a quote's `> ` and a list's marker. A blank line copies as an empty one. The
/// lines are joined with `\n`, and the copy ends without one.
pub fn markdown_of(bytes: &[u8], pieces: &[Piece]) -> Result<String, CopyError> {
    let text = decode(bytes);
    let lines = lines(&text).collect::<Vec<_>>();
    if pieces.is_empty() {
        return Ok(lines.join("\n"));
    }
    let mut copied = Vec::new();
    for &piece in pieces {
        match piece {
            Piece::Lines { first, last } => {
                copied.extend_from_slice(lines_in(&lines, piece, first, last)?);
            }
            Piece::Span { line, first, last } => {
                copied.push(span_in(&lines, piece, line, first, last)?);
            }
        }
    }
    let shared = shared_indentation(&copied);
    let unindented = copied.iter().map(|line| {
        if is_blank(line) {
            ""
        } else {
            line.strip_prefix(shared)
                .unwrap_or_else(|| line.trim_start_matches(INDENTATION))
        }
    });
    Ok(unindented.collect::<Vec<_>>().join("\n"))
}

/// Whether `line` is empty or only indentation.
fn is_blank(line: &str) -> bool {
    line.trim_start_matches(INDENTATION).is_empty()
}

/// What Markdown counts as indentation.
const INDENTATION: [char; 2] = [' ', '\t'];

/// The lines from `first` to `last` among `lines`, which `piece` names.
fn lines_in<'a, 'text>(
    lines: &'a [&'text str],
    piece: Piece,
    first: usize,
    last: usize,
) -> Result<&'a [&'text str], CopyError> {
    if first == 0 || first > last {
        return Err(CopyError::NotInTheFile(piece));
    }
    lines
        .get(first - 1..last)
        .ok_or(CopyError::NotInTheFile(piece))
}

/// The columns from `first` to `last` of the line `line` among `lines`, which `piece` names,
/// without the spaces around them: a table cell's line numbers take in the spaces between its
/// pipes and its text.
fn span_in<'text>(
    lines: &[&'text str],
    piece: Piece,
    line: usize,
    first: usize,
    last: usize,
) -> Result<&'text str, CopyError> {
    let text = line
        .checked_sub(1)
        .and_then(|index| lines.get(index))
        .ok_or(CopyError::NotInTheFile(piece))?;
    if first == 0 || first > last || last > text.len() {
        return Err(CopyError::NotInTheFile(piece));
    }
    let span = text
        .get(first - 1..last)
        .ok_or(CopyError::SplitsACharacter { line, first, last })?;
    Ok(span.trim_matches(INDENTATION))
}

/// The indentation every line of `lines` that isn't blank starts with.
fn shared_indentation<'text>(lines: &[&'text str]) -> &'text str {
    let mut indented = lines
        .iter()
        .filter(|line| !is_blank(line))
        .map(|line| &line[..line.len() - line.trim_start_matches(INDENTATION).len()]);
    let Some(first) = indented.next() else {
        return "";
    };
    indented.fold(first, |shared, indentation| {
        let common = shared
            .bytes()
            .zip(indentation.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        &shared[..common]
    })
}
