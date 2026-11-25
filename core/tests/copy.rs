//! Copy as Markdown's core (copy-as-markdown tickets 01 to 03): the Markdown of the pieces of a
//! file the page names. Each piece's lines and columns are taken from the core's own render, as
//! the page takes them from the rendered blocks, so the two can't drift apart.

mod common;

use std::error::Error;
use std::path::Path;

use common::Nothing;
use downlight_core::{CopyError, Piece, markdown_of};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The core's HTML for `bytes`.
fn rendered(bytes: &[u8]) -> TestResult<String> {
    Ok(downlight_core::render(
        bytes,
        Path::new("a.md"),
        "http://image.localhost",
        Nothing,
        &downlight_core::Leave::new(),
    )?
    .as_str()
    .to_owned())
}

/// A point of the file as `data-sourcepos` names it: its line and column.
type Point = (usize, usize);

/// Where the `nth` element (from 0) whose tag is `tag` in the core's render of `bytes` starts and
/// ends in the file, by its `data-sourcepos` (`line:column-line:column`).
fn sourcepos(bytes: &[u8], tag: &str, nth: usize) -> TestResult<(Point, Point)> {
    let html = rendered(bytes)?;
    let opening = format!("<{tag} ");
    let (at, _) = html
        .match_indices(&opening)
        .nth(nth)
        .ok_or_else(|| format!("no <{tag}> number {nth} in {html}"))?;
    let tag_text = &html[at..at + html[at..].find('>').ok_or("an unclosed tag")?];
    let (_, sourcepos) = tag_text
        .split_once("data-sourcepos=\"")
        .ok_or_else(|| format!("{tag_text} has no line numbers"))?;
    let sourcepos = sourcepos.split('"').next().ok_or("an unclosed attribute")?;
    let (start, end) = sourcepos
        .split_once('-')
        .ok_or("a sourcepos without an end")?;
    let point = |point: &str| -> TestResult<Point> {
        let (line, column) = point.split_once(':').ok_or("a point without a column")?;
        Ok((line.parse()?, column.parse()?))
    };
    Ok((point(start)?, point(end)?))
}

/// The lines of the `nth` block (from 0) whose tag is `tag`, as a piece.
fn block(bytes: &[u8], tag: &str, nth: usize) -> TestResult<Piece> {
    let ((first, _), (last, _)) = sourcepos(bytes, tag, nth)?;
    Ok(Piece::Lines { first, last })
}

/// The span of the `nth` table cell (from 0) whose tag is `tag` (`th` or `td`), as the page sends
/// it for a selection within that cell.
fn cell(bytes: &[u8], tag: &str, nth: usize) -> TestResult<Piece> {
    let ((line, first), (_, last)) = sourcepos(bytes, tag, nth)?;
    Ok(Piece::Span { line, first, last })
}

/// The piece from the start of `first` to the end of `last`.
fn from_to(first: Piece, last: Piece) -> TestResult<Piece> {
    let (Piece::Lines { first, .. }, Piece::Lines { last, .. }) = (first, last) else {
        return Err("only ranges of lines run from one to another".into());
    };
    Ok(Piece::Lines { first, last })
}

#[test]
fn a_range_of_lines_copies_those_lines_of_the_file_as_written() -> TestResult {
    let markdown =
        b"# Title\n\nFirst *paragraph*,\nwrapped.\n\nA [second](b.md) one.\n\nThe end.\n";
    let pieces = [from_to(block(markdown, "p", 0)?, block(markdown, "p", 1)?)?];
    assert_eq!(
        markdown_of(markdown, &pieces)?,
        "First *paragraph*,\nwrapped.\n\nA [second](b.md) one."
    );
    Ok(())
}

#[test]
fn no_pieces_copy_the_whole_file() -> TestResult {
    let markdown = b"# Title\n\nSome text.\n";
    assert_eq!(markdown_of(markdown, &[])?, "# Title\n\nSome text.");
    Ok(())
}

#[test]
fn pieces_are_copied_in_the_order_given_and_joined_by_line_breaks() -> TestResult {
    let markdown = b"One.\n\nTwo.\n\nThree.\n";
    let pieces = [block(markdown, "p", 0)?, block(markdown, "p", 2)?];
    assert_eq!(markdown_of(markdown, &pieces)?, "One.\nThree.");
    Ok(())
}

#[test]
fn windows_line_endings_copy_as_line_breaks() -> TestResult {
    let markdown = b"# Title\r\n\r\nFirst line\r\nsecond line\r\n";
    let pieces = [block(markdown, "p", 0)?];
    assert_eq!(markdown_of(markdown, &pieces)?, "First line\nsecond line");
    Ok(())
}

#[test]
fn a_lone_carriage_return_ends_a_line_as_it_does_for_the_render() -> TestResult {
    let markdown = b"# Title\r\rFirst line\rsecond line\r";
    let pieces = [block(markdown, "p", 0)?];
    assert_eq!(markdown_of(markdown, &pieces)?, "First line\nsecond line");
    Ok(())
}

#[test]
fn a_nested_list_item_loses_the_indentation_its_lines_share_and_keeps_its_marker() -> TestResult {
    let markdown = b"- Pack\n  - [ ] Cards\n    - Spare ones\n- Print\n";
    let pieces = [block(markdown, "li", 1)?];
    assert_eq!(
        markdown_of(markdown, &pieces)?,
        "- [ ] Cards\n  - Spare ones"
    );
    Ok(())
}

#[test]
fn blank_lines_are_ignored_when_finding_the_indentation_lines_share() -> TestResult {
    let markdown = b"- Pack\n  - Cards\n\n    More about cards.\n";
    let pieces = [block(markdown, "li", 1)?];
    assert_eq!(
        markdown_of(markdown, &pieces)?,
        "- Cards\n\n  More about cards."
    );
    Ok(())
}

#[test]
fn a_quote_keeps_its_markers() -> TestResult {
    let markdown = b"Before.\n\n> Quoted,\n> and more.\n";
    let pieces = [block(markdown, "blockquote", 0)?];
    assert_eq!(markdown_of(markdown, &pieces)?, "> Quoted,\n> and more.");
    Ok(())
}

#[test]
fn lines_that_share_no_indentation_keep_theirs() -> TestResult {
    let markdown = b"Text.\n\n    indented code\n";
    let pieces = [from_to(
        block(markdown, "p", 0)?,
        block(markdown, "pre", 0)?,
    )?];
    assert_eq!(
        markdown_of(markdown, &pieces)?,
        "Text.\n\n    indented code"
    );
    Ok(())
}

#[test]
fn a_utf_16_file_copies_as_the_characters_it_shows() -> TestResult {
    // "# Größe\r\n\r\nCafé\r\n" in UTF-16 LE with its byte order mark, as Windows PowerShell 5.1's
    // Out-File writes it.
    let bytes = [
        0xff, 0xfe, 0x23, 0x00, 0x20, 0x00, 0x47, 0x00, 0x72, 0x00, 0xf6, 0x00, 0xdf, 0x00, 0x65,
        0x00, 0x0d, 0x00, 0x0a, 0x00, 0x0d, 0x00, 0x0a, 0x00, 0x43, 0x00, 0x61, 0x00, 0x66, 0x00,
        0xe9, 0x00, 0x0d, 0x00, 0x0a, 0x00,
    ];
    let pieces = [from_to(block(&bytes, "h1", 0)?, block(&bytes, "p", 0)?)?];
    assert_eq!(markdown_of(&bytes, &pieces)?, "# Größe\n\nCafé");
    Ok(())
}

#[test]
fn a_windows_1252_file_copies_as_the_characters_it_shows() -> TestResult {
    // "Größe: 5 €" in Windows-1252, which is no valid UTF-8.
    let bytes = b"# Title\n\nGr\xf6\xdfe: 5 \x80\n";
    let pieces = [block(bytes, "p", 0)?];
    assert_eq!(markdown_of(bytes, &pieces)?, "Größe: 5 €");
    Ok(())
}

#[test]
fn a_utf_8_byte_order_mark_never_reaches_the_copy() -> TestResult {
    let bytes = b"\xef\xbb\xbf# Title\n";
    assert_eq!(markdown_of(bytes, &[])?, "# Title");
    Ok(())
}

#[test]
fn front_matter_copies_as_it_is_written() -> TestResult {
    let markdown = b"---\ntitle: Notes\n---\n\n# Notes\n";
    let pieces = [from_to(
        block(markdown, "pre", 0)?,
        block(markdown, "h1", 0)?,
    )?];
    assert_eq!(
        markdown_of(markdown, &pieces)?,
        "---\ntitle: Notes\n---\n\n# Notes"
    );
    Ok(())
}

#[test]
fn a_piece_outside_the_file_is_an_error() {
    let markdown = b"One.\n\nTwo.\n";
    for piece in [
        Piece::Lines { first: 3, last: 4 },
        Piece::Lines { first: 0, last: 1 },
        Piece::Lines { first: 2, last: 1 },
    ] {
        assert_eq!(
            markdown_of(markdown, &[piece]),
            Err(CopyError::NotInTheFile(piece)),
            "{piece:?}"
        );
    }
}

/// The first and last line of the `nth` block (from 0) whose tag is `tag`.
fn bounds(bytes: &[u8], tag: &str, nth: usize) -> TestResult<(usize, usize)> {
    let ((first, _), (last, _)) = sourcepos(bytes, tag, nth)?;
    Ok((first, last))
}

/// The line `offset` lines after the first of the `nth` block (from 0) whose tag is `tag`, as a
/// piece of that one line. A code block's lines are its opening fence, its code and its closing
/// fence, so the page names the part of one it copies by these.
fn line_of(bytes: &[u8], tag: &str, nth: usize, offset: usize) -> TestResult<Piece> {
    let (first, _) = bounds(bytes, tag, nth)?;
    Ok(Piece::Lines {
        first: first + offset,
        last: first + offset,
    })
}

/// The block's last line as a piece of that one line.
fn last_line_of(bytes: &[u8], tag: &str, nth: usize) -> TestResult<Piece> {
    let (_, last) = bounds(bytes, tag, nth)?;
    Ok(Piece::Lines { first: last, last })
}

#[test]
fn a_fenced_code_blocks_touched_lines_copy_between_its_opening_and_closing_fences() -> TestResult {
    let markdown = b"Text.\n\n```rust\nlet a = 1;\nlet b = 2;\nlet c = 3;\n```\n\nAfter.\n";
    let pieces = [
        line_of(markdown, "pre", 0, 0)?,
        line_of(markdown, "pre", 0, 2)?,
        line_of(markdown, "pre", 0, 3)?,
        last_line_of(markdown, "pre", 0)?,
    ];
    assert_eq!(
        markdown_of(markdown, &pieces)?,
        "```rust\nlet b = 2;\nlet c = 3;\n```"
    );
    Ok(())
}

#[test]
fn code_lines_touched_from_a_paragraph_copy_with_the_paragraph_and_the_closing_fence() -> TestResult
{
    let markdown = b"Text,\nwrapped.\n\n```rust\nlet a = 1;\nlet b = 2;\nlet c = 3;\n```\n";
    let (first, _) = bounds(markdown, "p", 0)?;
    let (fence, _) = bounds(markdown, "pre", 0)?;
    let pieces = [
        // The paragraph through the code line touched, then the closing fence.
        Piece::Lines {
            first,
            last: fence + 2,
        },
        last_line_of(markdown, "pre", 0)?,
    ];
    assert_eq!(
        markdown_of(markdown, &pieces)?,
        "Text,\nwrapped.\n\n```rust\nlet a = 1;\nlet b = 2;\n```"
    );
    Ok(())
}

#[test]
fn a_fenced_block_in_a_list_item_copies_without_the_indentation_its_lines_share() -> TestResult {
    let markdown = b"1. Install it:\n\n   ```sh\n   npm install\n   npm test\n   ```\n";
    let pieces = [
        line_of(markdown, "pre", 0, 0)?,
        line_of(markdown, "pre", 0, 2)?,
        last_line_of(markdown, "pre", 0)?,
    ];
    assert_eq!(markdown_of(markdown, &pieces)?, "```sh\nnpm test\n```");
    Ok(())
}

#[test]
fn front_matter_copies_between_its_delimiter_lines() -> TestResult {
    let markdown = b"---\ntitle: Notes\ntags: [a, b]\n---\n\n# Notes\n";
    let pieces = [
        line_of(markdown, "pre", 0, 0)?,
        line_of(markdown, "pre", 0, 2)?,
        last_line_of(markdown, "pre", 0)?,
    ];
    assert_eq!(markdown_of(markdown, &pieces)?, "---\ntags: [a, b]\n---");
    Ok(())
}

#[test]
fn the_touched_lines_of_an_indented_code_block_copy_with_no_fences() -> TestResult {
    let markdown = b"Text.\n\n    one\n\n    two\n    three\n";
    let (first, _) = bounds(markdown, "p", 0)?;
    let (code, _) = bounds(markdown, "pre", 0)?;
    // The paragraph through the third code line: all of an indented block's lines are code.
    let pieces = [Piece::Lines {
        first,
        last: code + 2,
    }];
    assert_eq!(
        markdown_of(markdown, &pieces)?,
        "Text.\n\n    one\n\n    two"
    );
    Ok(())
}

#[test]
fn an_unclosed_fence_at_the_end_of_a_document_copies_with_only_its_opening_line() -> TestResult {
    let markdown = b"Text.\n\n```text\nnever closed\nstill code\nand more\n";
    let pieces = [
        line_of(markdown, "pre", 0, 0)?,
        line_of(markdown, "pre", 0, 2)?,
    ];
    assert_eq!(markdown_of(markdown, &pieces)?, "```text\nstill code");
    Ok(())
}

// Copy as Markdown of part of a table (copy-as-markdown ticket 03): a cell's span, and a table's
// header, delimiter and touched rows, from the pieces the page would send.

/// A table whose second row's note has emphasis and a link, and whose prices end in a character
/// of more than one byte.
const PARTS: &[u8] = b"The parts:\n\n\
| Part | Note | Price |\n\
| --- | --- | ---: |\n\
| Wick | Trim it before each lamp is lit | 2 \xe2\x82\xac |\n\
| Glass |   Clean it *when* the [lamp](lamp.md) is cold   | 9 \xe2\x82\xac |\n\
| Oil | Fill it to the line | 4 \xe2\x82\xac |\n\
| Base | Keep it level | 12 \xe2\x82\xac |\n\
\n\
After the table.\n";

#[test]
fn a_cells_span_copies_its_markdown_without_the_spaces_around_it() -> TestResult {
    // The second row's note: the fifth `td`.
    let pieces = [cell(PARTS, "td", 4)?];
    assert_eq!(
        markdown_of(PARTS, &pieces)?,
        "Clean it *when* the [lamp](lamp.md) is cold"
    );
    Ok(())
}

#[test]
fn a_cells_span_counts_its_columns_in_the_bytes_of_its_line() -> TestResult {
    let prices = [cell(PARTS, "td", 2)?, cell(PARTS, "td", 11)?];
    assert_eq!(markdown_of(PARTS, &prices[..1])?, "2 €");
    assert_eq!(markdown_of(PARTS, &prices[1..])?, "12 €");
    Ok(())
}

#[test]
fn a_header_cells_span_copies_its_markdown() -> TestResult {
    let pieces = [cell(PARTS, "th", 1)?];
    assert_eq!(markdown_of(PARTS, &pieces)?, "Note");
    Ok(())
}

#[test]
fn a_cells_span_in_a_quoted_table_copies_without_the_quotes_marker() -> TestResult {
    let markdown = "> | Größe | Wert |\n> | --- | --- |\n> | Fuß | *lang* |\n".as_bytes();
    let pieces = [cell(markdown, "td", 1)?];
    assert_eq!(markdown_of(markdown, &pieces)?, "*lang*");
    Ok(())
}

#[test]
fn a_cells_span_in_a_windows_1252_file_copies_as_the_characters_it_shows() -> TestResult {
    // "| Größe | 5 € |" in Windows-1252: the columns are counted in the decoded text, as the render
    // counts them.
    let bytes = b"| Gr\xf6\xdfe | Preis |\n| --- | --- |\n| Gr\xf6\xdfe | 5 \x80 |\n";
    assert_eq!(markdown_of(bytes, &[cell(bytes, "td", 0)?])?, "Größe");
    assert_eq!(markdown_of(bytes, &[cell(bytes, "td", 1)?])?, "5 €");
    Ok(())
}

#[test]
fn a_tables_header_delimiter_and_touched_rows_copy_as_a_table_of_only_those_rows() -> TestResult {
    let (header, _) = bounds(PARTS, "tr", 0)?;
    let row = |nth: usize| -> TestResult<Piece> {
        let (line, _) = bounds(PARTS, "tr", nth)?;
        Ok(Piece::Lines {
            first: line,
            last: line,
        })
    };
    // From the second row to the fourth, its third left out, as when a sort shows the two side
    // by side.
    let pieces = [
        Piece::Lines {
            first: header,
            last: header + 1,
        },
        row(2)?,
        row(4)?,
    ];
    assert_eq!(
        markdown_of(PARTS, &pieces)?,
        "| Part | Note | Price |\n\
         | --- | --- | ---: |\n\
         | Glass |   Clean it *when* the [lamp](lamp.md) is cold   | 9 € |\n\
         | Base | Keep it level | 12 € |"
    );
    Ok(())
}

#[test]
fn a_paragraph_into_a_table_copies_with_the_tables_header_delimiter_and_touched_rows() -> TestResult
{
    let (paragraph, _) = bounds(PARTS, "p", 0)?;
    let (first_row, _) = bounds(PARTS, "tr", 1)?;
    // The paragraph through the first row, as one range: the header and delimiter rows lie
    // between them.
    let pieces = [Piece::Lines {
        first: paragraph,
        last: first_row,
    }];
    assert_eq!(
        markdown_of(PARTS, &pieces)?,
        "The parts:\n\n\
         | Part | Note | Price |\n\
         | --- | --- | ---: |\n\
         | Wick | Trim it before each lamp is lit | 2 € |"
    );
    Ok(())
}

#[test]
fn a_span_inside_a_character_is_an_error() -> TestResult {
    let Piece::Span { line, first, last } = cell(PARTS, "td", 2)? else {
        return Err("a cell's piece is a span".into());
    };
    // The cell is " 2 € ", whose euro sign takes three bytes from the fourth column on.
    for (first, last) in [(first + 4, last), (first, first + 3)] {
        assert_eq!(
            markdown_of(PARTS, &[Piece::Span { line, first, last }]),
            Err(CopyError::SplitsACharacter { line, first, last }),
            "columns {first} to {last}"
        );
    }
    Ok(())
}

#[test]
fn a_span_outside_its_line_is_an_error() -> TestResult {
    let Piece::Span { line, first, .. } = cell(PARTS, "td", 0)? else {
        return Err("a cell's piece is a span".into());
    };
    let length = "| Wick | Trim it before each lamp is lit | 2 € |".len();
    for piece in [
        Piece::Span {
            line,
            first: 0,
            last: 3,
        },
        Piece::Span {
            line,
            first,
            last: length + 1,
        },
        Piece::Span {
            line,
            first: 4,
            last: 3,
        },
        Piece::Span {
            line: 0,
            first: 1,
            last: 1,
        },
        Piece::Span {
            line: 99,
            first: 1,
            last: 1,
        },
    ] {
        assert_eq!(
            markdown_of(PARTS, &[piece]),
            Err(CopyError::NotInTheFile(piece)),
            "{piece:?}"
        );
    }
    Ok(())
}

#[test]
fn no_pieces_copy_a_file_indented_throughout_as_it_is_written() -> TestResult {
    let markdown = b"    one\n    two\n\n      three\n";
    assert_eq!(
        markdown_of(markdown, &[])?,
        "    one\n    two\n\n      three"
    );
    Ok(())
}

#[test]
fn a_blank_line_with_spaces_copies_as_an_empty_line() -> TestResult {
    let markdown = b"- Pack\n  - Cards\n      \n    More about cards.\n";
    let pieces = [block(markdown, "li", 1)?];
    assert_eq!(
        markdown_of(markdown, &pieces)?,
        "- Cards\n\n  More about cards."
    );
    Ok(())
}
