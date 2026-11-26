//! The document's counts (glossary: Counts, window design ticket 10): its lines and words, made
//! while it renders, on the render's background task, so that a large document doesn't hold up
//! the page and a reload brings new ones. The page counts only the selection.

use unicode_segmentation::UnicodeSegmentation;

use crate::lines::lines;
use crate::render::Leave;
use crate::sanitize::BLOCK_TAGS;

/// A document's lines and words, which the title bar's file name tooltip shows under its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Counts {
    /// The file's lines, as line numbers count them: one per line break, plus one when the last
    /// line has none. An empty file has none.
    pub lines: usize,
    /// The words of the text the reader sees, by UAX #29's word boundaries: headings, paragraphs,
    /// lists, tables, code, footnote labels and the text of HTML that survives sanitizing, but not
    /// image alt text or link targets, which aren't shown.
    pub words: usize,
}

/// The counts of a document whose decoded `text` rendered as the sanitized `html`, or None once
/// `leave` says nobody waits for the render any more (ticket 38).
pub(crate) fn counts_of(text: &str, html: &str, leave: &Leave) -> Option<Counts> {
    Some(Counts {
        lines: lines(text).count(),
        words: words_shown_by(html, leave)?,
    })
}

/// The words of the text `html` shows. The text is read from the sanitized HTML, not from the
/// Markdown, so that only what survives sanitizing counts, and alt text and link targets, which
/// are attributes, never do.
///
/// The HTML is html5ever's serialization, which ammonia writes: every attribute value is quoted
/// with `"`, and its text escapes `&`, `<`, `>` and the no-break space but leaves `"` as it is, so
/// a tag is read by skipping quoted values and every `&` starts an entity. A block's tag ends a
/// run of text, as a browser lays it out on a line of its own, and an inline tag such as `<em>`
/// doesn't, so `H<sub>2</sub>O` is one word, as the page's selection counts it
/// (`page/src/counts.ts`). Words never run across a block's edge, so each run is counted on its
/// own, which keeps UAX #29's ASCII path for each run of ASCII text even in a document with other
/// letters elsewhere.
///
/// A 100 MB document takes a while to count, so the count looks at `leave` after each run and
/// gives up with None once it is left.
fn words_shown_by(html: &str, leave: &Leave) -> Option<usize> {
    let mut words = 0;
    let mut run = String::new();
    let mut rest = html;
    while let Some(at) = rest.find(['<', '&']) {
        run.push_str(&rest[..at]);
        rest = &rest[at..];
        if rest.starts_with('<') {
            let end = tag_end(rest);
            if breaks_text(&rest[..end]) {
                words += run.unicode_words().count();
                run.clear();
                if leave.is_left() {
                    return None;
                }
            }
            rest = &rest[end..];
        } else {
            // None of the escaped characters joins a word under UAX #29, so an entity ends a word
            // as a space does.
            let end = rest.find(';').map_or(rest.len(), |end| end + 1);
            run.push(' ');
            rest = &rest[end..];
        }
    }
    run.push_str(rest);
    words += run.unicode_words().count();
    if leave.is_left() {
        return None;
    }
    Some(words)
}

/// Where the tag at the start of `html` ends, just after its `>`, skipping its quoted values.
fn tag_end(html: &str) -> usize {
    let mut quoted = false;
    for (at, byte) in html.bytes().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b'>' if !quoted => return at + 1,
            _ => {}
        }
    }
    html.len()
}

/// Whether `tag`, a start or end tag, puts what follows it on another line: a block's, or a line
/// break. The blocks are those sanitizing keeps, as the page's Find and selection counts know them
/// (`BLOCK_ELEMENTS` in `page/src/document-text.ts`).
fn breaks_text(tag: &str) -> bool {
    let name = tag
        .trim_start_matches('<')
        .trim_start_matches('/')
        .split(|character: char| character.is_ascii_whitespace() || matches!(character, '/' | '>'))
        .next()
        .unwrap_or_default();
    name == "br" || BLOCK_TAGS.contains(&name)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Counts;
    use crate::files::fake::FakeFiles;
    use crate::{Leave, render};

    fn counts_of(markdown: &str) -> Counts {
        render(
            markdown.as_bytes(),
            Path::new("README.md"),
            "http://image.localhost",
            FakeFiles::default(),
            &Leave::new(),
        )
        .unwrap()
        .counts()
    }

    fn lines_of(markdown: &str) -> usize {
        counts_of(markdown).lines
    }

    fn words_of(markdown: &str) -> usize {
        counts_of(markdown).words
    }

    #[test]
    fn a_last_line_counts_with_or_without_its_line_break() {
        assert_eq!(lines_of("a\nb"), 2);
        assert_eq!(lines_of("a\nb\n"), 2);
    }

    #[test]
    fn an_empty_file_has_no_lines_and_no_words() {
        assert_eq!(counts_of(""), Counts { lines: 0, words: 0 });
    }

    #[test]
    fn blank_lines_count_as_lines() {
        assert_eq!(lines_of("# Title\n\n\nText.\n"), 4);
    }

    #[test]
    fn windows_and_old_mac_line_endings_count_as_line_numbers_do() {
        assert_eq!(lines_of("a\r\nb\r\n"), 2);
        assert_eq!(lines_of("a\rb\rc"), 3);
    }

    #[test]
    fn utf_16_file_counts_its_lines_as_text() {
        // "a\nb\n" in UTF-16 LE with a byte order mark.
        let bytes = [0xff, 0xfe, b'a', 0, b'\n', 0, b'b', 0, b'\n', 0];
        let rendered = render(
            &bytes,
            Path::new("README.md"),
            "http://image.localhost",
            FakeFiles::default(),
            &Leave::new(),
        )
        .unwrap();
        assert_eq!(rendered.counts(), Counts { lines: 2, words: 2 });
    }

    #[test]
    fn words_of_headings_paragraphs_and_lists_count() {
        assert_eq!(
            words_of("# Getting started\n\nRead this first.\n\n- one item\n- two\n"),
            8
        );
    }

    #[test]
    fn words_of_a_table_count_cell_by_cell() {
        assert_eq!(
            words_of("| Option | Meaning |\n| --- | --- |\n| size | Card size |\n"),
            5
        );
    }

    // comrak writes a line break between its own blocks, so only raw HTML has blocks that touch.

    #[test]
    fn words_of_raw_html_blocks_that_touch_count_block_by_block() {
        assert_eq!(words_of("<div>one</div><div>two</div>\n"), 2);
    }

    #[test]
    fn words_of_raw_html_table_cells_that_touch_count_cell_by_cell() {
        assert_eq!(
            words_of("<table><tr><td>one</td><td>two</td></tr></table>\n"),
            2
        );
    }

    #[test]
    fn line_break_parts_the_words_around_it() {
        assert_eq!(words_of("one<br>two\n"), 2);
    }

    #[test]
    fn words_of_code_count_highlighted_or_not() {
        assert_eq!(words_of("Use `npm install` here.\n"), 4);
        assert_eq!(words_of("```rust\nlet answer = 42;\n```\n"), 3);
        assert_eq!(words_of("```\nplain words\n```\n"), 2);
    }

    #[test]
    fn inline_markup_inside_a_word_keeps_it_one_word() {
        assert_eq!(words_of("H<sub>2</sub>O and **bold**ly\n"), 3);
    }

    #[test]
    fn image_alt_text_and_link_targets_are_not_counted() {
        assert_eq!(
            words_of("![A long alt text](a.png) See [the guide](docs/guide.md).\n"),
            3
        );
        assert_eq!(
            words_of("<img alt=\"Alt words &quot;quoted&quot; > here\" src=\"a.png\"> Text\n"),
            1
        );
    }

    #[test]
    fn footnote_labels_count_as_words() {
        // "Text", its reference's label "1", and the note's "The note".
        assert_eq!(words_of("Text.[^n]\n\n[^n]: The note.\n"), 4);
    }

    #[test]
    fn html_removed_by_sanitizing_is_not_counted() {
        assert_eq!(
            words_of(
                "Kept <script>three removed words</script>text.\n\n<style>p { a: b }</style>\n"
            ),
            2
        );
    }

    #[test]
    fn text_of_html_that_survives_sanitizing_counts() {
        assert_eq!(
            words_of("<details>\n<summary>More here</summary>\n\nHidden text.\n\n</details>\n"),
            4
        );
    }

    #[test]
    fn escaped_character_ends_a_word_as_a_space_does() {
        // The HTML holds "a&amp;b" for both: an entity is one, even where the Markdown wrote it.
        assert_eq!(words_of("a&b\n"), 2);
        assert_eq!(words_of("a&amp;b\n"), 2);
        assert_eq!(words_of("AT&T and a<b and one&nbsp;two\n"), 8);
    }

    #[test]
    fn count_of_a_left_render_gives_up() {
        let leave = Leave::new();
        leave.leave();
        assert_eq!(
            super::counts_of("one\n\ntwo\n", "<p>one</p>\n<p>two</p>\n", &leave),
            None
        );
    }

    #[test]
    fn chinese_text_counts_by_uax_29() {
        // UAX #29 without a dictionary parts Han text at every character.
        assert_eq!(words_of("你好世界\n"), 4);
        assert_eq!(words_of("Downlight 是一个查看器\n"), 7);
    }

    #[test]
    fn front_matter_counts_as_the_code_it_shows_as() {
        assert_eq!(
            counts_of("---\ntitle: Notes\n---\nText\n"),
            Counts { lines: 4, words: 3 }
        );
    }
}
