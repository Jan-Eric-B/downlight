//! Sanitizing: the rendered HTML keeps what GitHub allows in a document and loses everything that
//! could act on its own (spec: Render). All of the HTML is sanitized, comrak's own markup too,
//! since a document's raw HTML and link targets pass through it.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use ammonia::Builder;
use comrak::nodes::{Node, NodeValue};

use crate::file_url::{self, Target};
use crate::files::FileSystem;
use crate::highlight::is_highlight_class;
use crate::image::ImageSources;
use crate::links::goes_to_the_system;

// The elements GitHub allows in a document, and the ones rendering writes besides: `section`
// around the footnotes and `input` for task list checkboxes. They are the block tags and the
// inline tags together.

/// The block elements a document can hold: each starts on a line of its own, so the text before
/// it and after it never runs together, as the document's words count it (`counts.rs`).
pub(crate) const BLOCK_TAGS: &[&str] = &[
    "blockquote",
    "caption",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "li",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];

/// The inline elements a document can hold, which sit within a line of text.
const INLINE_TAGS: &[&str] = &[
    "a", "abbr", "b", "bdo", "br", "cite", "code", "del", "dfn", "em", "i", "img", "input", "ins",
    "kbd", "mark", "picture", "q", "rp", "rt", "ruby", "s", "samp", "small", "source", "span",
    "strike", "strong", "sub", "sup", "time", "tt", "var", "wbr",
];

/// Attributes allowed on every element: GitHub's presentational ones, and the ids, classes and
/// line numbers (`data-sourcepos`) that rendering writes.
const GENERIC_ATTRIBUTES: &[&str] = &[
    "align",
    "aria-hidden",
    "aria-label",
    "class",
    "data-sourcepos",
    "dir",
    "height",
    "id",
    "lang",
    "title",
    "width",
];

/// Attributes allowed on one element only.
const TAG_ATTRIBUTES: &[(&str, &[&str])] = &[
    (
        "a",
        &[
            "href",
            "name",
            // Rendering's footnotes and heading anchors; github-markdown-css styles
            // `data-footnote-ref`.
            "data-footnote-ref",
            "data-footnote-backref",
            "data-footnote-backref-idx",
            "data-heading-content",
        ],
    ),
    ("details", &["open"]),
    ("img", &["src", "alt"]),
    ("input", &["type", "checked", "disabled"]),
    ("ol", &["start", "type"]),
    ("section", &["data-footnotes"]),
    ("source", &["srcset", "media", "type"]),
    ("td", &["colspan", "rowspan", "valign"]),
    ("th", &["colspan", "rowspan", "valign"]),
    ("time", &["datetime"]),
];

/// The classes rendering writes, which github-markdown-css styles. A document may use these and no
/// others, so that it can't take on the look of the page's own parts, such as its messages.
const CLASSES: &[&str] = &[
    "anchor",
    "contains-task-list",
    "footnote-backref",
    "footnote-ref",
    "footnotes",
    "markdown-alert",
    "markdown-alert-caution",
    "markdown-alert-important",
    "markdown-alert-note",
    "markdown-alert-tip",
    "markdown-alert-title",
    "markdown-alert-warning",
    "task-list-item",
    "task-list-item-checkbox",
];

/// The class that names a code block's language, such as `language-rust`.
const LANGUAGE_CLASS: &str = "language-";

/// Every scheme that `href` and `src` may have before the attribute filter decides on them.
/// ammonia removes a URL whose scheme isn't listed before it calls the filter, so this lets
/// through what the filter judges: links' schemes, `data:` images and drive letters, which the URL
/// parser reads as a scheme (`C:\docs` has the scheme `c`).
const SCHEMES_TO_FILTER: &[&str] = &[
    "http", "https", "mailto", "file", "data", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j",
    "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
];

/// The tags a document's raw HTML may not use: those GitHub's tagfilter escapes, which change how
/// the HTML after them is read (an unclosed `<textarea>` turns the rest of the document into its
/// text), `noscript`, which does too while scripting is on, as it is for ammonia's parser, and
/// `template`, whose contents an unclosed one takes the rest of the document into and which
/// sanitizing drops with them. comrak 0.59 has no tagfilter (ticket 04).
const FILTERED_TAGS: [&str; 11] = [
    "title",
    "textarea",
    "style",
    "xmp",
    "iframe",
    "noembed",
    "noframes",
    "script",
    "plaintext",
    "noscript",
    "template",
];

/// The tags only rendering may write: task list checkboxes are rendering's, so a document can't
/// add form fields or task items of its own.
const RENDERING_ONLY_TAGS: [&str; 1] = ["input"];

/// A filtered tag is renamed with this prefix before the HTML is parsed. The parser reads the
/// new name like any unknown element, so nothing after it changes, and sanitizing drops it. The
/// document's own line numbers are renamed with it too, into an attribute sanitizing drops.
const DROPPED_PREFIX: &str = "dropped-";

/// The renamed tags whose content goes with them, as ammonia does for `script` and `style`.
const DROPPED_WITH_CONTENT: [&str; 2] = ["dropped-script", "dropped-style"];

/// The attribute rendering writes each block's line number in. Only rendering may write it: a
/// document's raw HTML that carried its own would give its blocks false line numbers, which
/// reload anchoring and Find's first match then use (ticket 34).
const LINE_NUMBER_ATTRIBUTE: &str = "data-sourcepos";

/// Renames, in the document's raw HTML, the filtered and the rendering-only tags and the line
/// number attributes, so that sanitizing drops them, and turns the markup a raw HTML node leaves
/// unfinished into text. Raw HTML is the only place they can come from: comrak escapes `<`
/// everywhere else, and writes its own line numbers only once this ran.
pub(crate) fn drop_from_raw_html<'a>(root: Node<'a>) {
    for node in root.descendants() {
        let mut data = node.data_mut();
        let raw = match &mut data.value {
            NodeValue::HtmlBlock(block) => &mut block.literal,
            NodeValue::HtmlInline(raw) => raw,
            _ => continue,
        };
        // The scan comes first, so that an unfinished filtered tag shows as the document wrote
        // it. Renaming after the scan moves no markup's end, so what the scan read still holds:
        // it only puts letters after a `<`.
        let scanned = edited(raw, line_numbers_and_unfinished_markup(raw));
        let scanned_raw = scanned.as_deref().unwrap_or(raw);
        let renames = filtered_tag_names(scanned_raw).map(|name| (name, Edit::Drop));
        if let Some(edited) = edited(scanned_raw, renames).or(scanned) {
            *raw = edited;
        }
    }
}

/// A change to raw HTML, at a byte offset on an ASCII character.
#[derive(Clone, Copy)]
enum Edit {
    /// [`DROPPED_PREFIX`] goes before the name that starts there.
    Drop,
    /// The `<` there is written as `&lt;`, so that it starts no markup and shows as text.
    Escape,
}

/// `raw` with the `edits` made, in the order of their offsets, or None when there are none.
fn edited(raw: &str, edits: impl IntoIterator<Item = (usize, Edit)>) -> Option<String> {
    let mut edited: Option<String> = None;
    let mut copied = 0;
    for (at, edit) in edits {
        let out =
            edited.get_or_insert_with(|| String::with_capacity(raw.len() + DROPPED_PREFIX.len()));
        out.push_str(&raw[copied..at]);
        copied = match edit {
            Edit::Drop => {
                out.push_str(DROPPED_PREFIX);
                at
            }
            Edit::Escape => {
                out.push_str("&lt;");
                at + 1
            }
        };
    }
    edited.map(|mut out| {
        out.push_str(&raw[copied..]);
        out
    })
}

/// Where the name of each filtered or rendering-only tag starts in `raw`.
fn filtered_tag_names(raw: &str) -> impl Iterator<Item = usize> {
    let bytes = raw.as_bytes();
    raw.match_indices('<').filter_map(|(at, _)| {
        let is_end_tag = bytes.get(at + 1) == Some(&b'/');
        // `<` and `/` are ASCII, so the name starts on a char boundary.
        let name_start = at + 1 + usize::from(is_end_tag);
        let mut dropped = FILTERED_TAGS.iter().chain(&RENDERING_ONLY_TAGS);
        dropped
            .any(|tag| names_tag(bytes, name_start, tag))
            .then_some(name_start)
    })
}

/// Whether `bytes` hold `tag`'s name from `start` on, in any case, ending where the HTML parser
/// ends a tag name. The end of `bytes` counts, since the HTML that follows is comrak's.
fn names_tag(bytes: &[u8], start: usize, tag: &str) -> bool {
    let end = start + tag.len();
    bytes
        .get(start..end)
        .is_some_and(|name| name.eq_ignore_ascii_case(tag.as_bytes()))
        && bytes
            .get(end)
            .is_none_or(|next| is_html_space(*next) || matches!(next, b'/' | b'>'))
}

/// The edits that drop each [`LINE_NUMBER_ATTRIBUTE`] of a start tag in `raw`, in any case, and
/// escape the markup `raw` leaves unfinished (ticket 34).
///
/// `raw` is read as the HTML parser's tokenizer reads it (WHATWG HTML, "Tokenization"): tags with
/// their quoted values, comments, DOCTYPEs, CDATA and bogus comments. So text, values and comments
/// that name the attribute stay as they are, and no tag the parser reads is missed. Raw text
/// elements need no reading of their own: each one is a filtered tag, renamed into one the parser
/// reads like any other.
///
/// Each node starts where the parser reads text, and ends there too: the markup it leaves
/// unfinished, such as a tag with an open quote or a comment without its end, is escaped with all
/// that follows it in the node, so that comrak's markup after the node can't become that tag's
/// attributes or that comment's text. This shows broken HTML as text. Every character that ends
/// markup is ASCII, so each offset is on a char boundary.
fn line_numbers_and_unfinished_markup(raw: &str) -> Vec<(usize, Edit)> {
    let bytes = raw.as_bytes();
    let mut edits = Vec::new();
    let mut cdata_close = FirstClose::default();
    let mut at = 0;
    while let Some(open) = position_from(bytes, at, b'<') {
        let after = open + 1;
        let rest = &bytes[after..];
        let end = match rest.first().copied() {
            Some(letter) if letter.is_ascii_alphabetic() => {
                let mut names = Vec::new();
                let end = tag_end(bytes, after, &mut names);
                if end.is_some() {
                    edits.extend(names.into_iter().map(|name| (name, Edit::Drop)));
                }
                end
            }
            Some(b'/') => match rest.get(1) {
                // An end tag's attributes are read as a start tag's, and then dropped.
                Some(letter) if letter.is_ascii_alphabetic() => {
                    tag_end(bytes, after + 1, &mut Vec::new())
                }
                // The parser drops `</>`, and reads `</` before anything else as a bogus comment.
                Some(b'>') => Some(after + 2),
                Some(_) => close_after(bytes, after + 1),
                None => None,
            },
            Some(b'!') if rest.starts_with(b"!--") => comment_end(bytes, after + 3),
            Some(b'!') if rest.starts_with(b"![CDATA[") => {
                let content = after + 8;
                match cdata_close.from(bytes, content) {
                    Some(close) if bytes[content..close].ends_with(b"]]") => Some(close + 1),
                    // Inside `svg` and `math` CDATA ends at `]]>`, elsewhere it is a bogus
                    // comment that ends at its first `>`. Where the two differ, the scan can't
                    // tell which the parser reads, so the CDATA shows as text and the parser
                    // reads on after its `<`, as the scan does.
                    _ => {
                        edits.push((open, Edit::Escape));
                        Some(after)
                    }
                }
            }
            // A DOCTYPE and a bogus comment end at their first `>`, even one inside quotes.
            Some(b'!' | b'?') => close_after(bytes, after),
            // A `<` before anything else is text.
            Some(_) => Some(after),
            None => None,
        };
        let Some(end) = end else {
            let unfinished = bytes[open..].iter().enumerate();
            edits.extend(
                unfinished
                    .filter(|(_, next)| **next == b'<')
                    .map(|(offset, _)| (open + offset, Edit::Escape)),
            );
            break;
        };
        at = end;
    }
    edits
}

/// Where the tag whose name starts at `name` ends, just after its `>`, or None when `bytes` end
/// inside it. Each of its attributes named [`LINE_NUMBER_ATTRIBUTE`] goes in `line_numbers`.
fn tag_end(bytes: &[u8], name: usize, line_numbers: &mut Vec<usize>) -> Option<usize> {
    let byte = |at: usize| bytes.get(at).copied();
    let mut at = name;
    while byte(at).is_some_and(|next| !ends_a_name(next)) {
        at += 1;
    }
    loop {
        while byte(at).is_some_and(|next| is_html_space(next) || next == b'/') {
            at += 1;
        }
        if byte(at)? == b'>' {
            return Some(at + 1);
        }
        let name_start = at;
        // A name's first character may be `=`, as the parser reads it.
        at += 1;
        while byte(at).is_some_and(|next| !ends_a_name(next) && next != b'=') {
            at += 1;
        }
        if bytes[name_start..at].eq_ignore_ascii_case(LINE_NUMBER_ATTRIBUTE.as_bytes()) {
            line_numbers.push(name_start);
        }
        let mut value = at;
        while byte(value).is_some_and(is_html_space) {
            value += 1;
        }
        if byte(value) != Some(b'=') {
            continue;
        }
        value += 1;
        while byte(value).is_some_and(is_html_space) {
            value += 1;
        }
        at = match byte(value) {
            Some(quote @ (b'"' | b'\'')) => position_from(bytes, value + 1, quote)? + 1,
            _ => {
                let mut end = value;
                while byte(end).is_some_and(|next| !is_html_space(next) && next != b'>') {
                    end += 1;
                }
                end
            }
        };
    }
}

/// Where the comment whose text starts at `text`, just after its `<!--`, ends, just after its
/// `>`, or None when `bytes` end inside it. As the parser reads it, `<!-->` and `<!--->` end at
/// once, and after them the first `>` that follows `--` or `--!` in the text ends it.
fn comment_end(bytes: &[u8], text: usize) -> Option<usize> {
    let rest = &bytes[text..];
    if rest.starts_with(b">") {
        return Some(text + 1);
    }
    if rest.starts_with(b"->") {
        return Some(text + 2);
    }
    let mut at = text;
    loop {
        let close = position_from(bytes, at, b'>')?;
        let before = &bytes[text..close];
        if before.ends_with(b"--") || before.ends_with(b"--!") {
            return Some(close + 1);
        }
        at = close + 1;
    }
}

/// Just after the first `>` from `from` on, or None when there is none.
fn close_after(bytes: &[u8], from: usize) -> Option<usize> {
    position_from(bytes, from, b'>').map(|close| close + 1)
}

/// The first `>` from an offset on, kept for the offsets after it up to the `>`, so that a run of
/// CDATA that all look for the same `>` reads up to it once, and the scan stays linear.
#[derive(Default)]
struct FirstClose {
    from: usize,
    found: Option<Option<usize>>,
}

impl FirstClose {
    fn from(&mut self, bytes: &[u8], from: usize) -> Option<usize> {
        match self.found {
            Some(found) if self.from <= from && found.is_none_or(|close| close >= from) => found,
            _ => {
                let found = position_from(bytes, from, b'>');
                self.from = from;
                self.found = Some(found);
                found
            }
        }
    }
}

/// Where the first `byte` from `from` on is, or None when there is none.
fn position_from(bytes: &[u8], from: usize, byte: u8) -> Option<usize> {
    let offset = bytes.get(from..)?.iter().position(|&next| next == byte)?;
    Some(from + offset)
}

/// Whether `byte` ends a tag's or an attribute's name, as the HTML parser reads it.
fn ends_a_name(byte: u8) -> bool {
    is_html_space(byte) || matches!(byte, b'/' | b'>')
}

/// HTML's whitespace, which separates a tag's name and its attributes.
fn is_html_space(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

/// Sanitizes the HTML that comrak rendered, with its images loading from `images`. Run
/// [`drop_from_raw_html`] on the document's tree before rendering it. The lookups are `Send`,
/// `Sync` and `'static` because ammonia asks that of its attribute filter, which holds them.
pub(crate) fn sanitize<F: FileSystem + Send + Sync + 'static>(
    html: &str,
    images: ImageSources<F>,
) -> String {
    let tag_attributes = TAG_ATTRIBUTES
        .iter()
        .map(|(tag, attributes)| (*tag, attributes.iter().copied().collect()))
        .collect::<HashMap<_, HashSet<_>>>();
    Builder::empty()
        .tags(BLOCK_TAGS.iter().chain(INLINE_TAGS).copied().collect())
        .generic_attributes(GENERIC_ATTRIBUTES.iter().copied().collect())
        .tag_attributes(tag_attributes)
        .url_schemes(SCHEMES_TO_FILTER.iter().copied().collect())
        .add_clean_content_tags(DROPPED_WITH_CONTENT)
        .attribute_filter(
            move |element, attribute, value| match (element, attribute) {
                // These are the only URL attributes the allowlist keeps.
                ("a", "href") => link_target(value),
                ("img", "src") => images.source(value).map(Cow::Owned),
                ("source", "srcset") => images.srcset(value).map(Cow::Owned),
                (_, "class") => allowed_classes(element, value),
                _ => Some(value.into()),
            },
        )
        // Links go through the core's link resolution, not the webview (ticket 11).
        .link_rel(None)
        .clean(html)
        .to_string()
}

/// `href` if a link may keep it: one naming a file or folder, or a URL that link resolution hands
/// to the system. A link with any other target, such as `javascript:`, loses it and does nothing,
/// as link resolution would have it do anyway; what a kept link does is decided when it is
/// clicked (ticket 11).
fn link_target(href: &str) -> Option<Cow<'_, str>> {
    let kept = match file_url::classify(href) {
        Target::File => true,
        Target::Url(url) => goes_to_the_system(&url),
        Target::Unusable => false,
    };
    kept.then_some(Cow::Borrowed(href))
}

/// The classes of `classes` that an `element` may have, or None for none.
fn allowed_classes<'v>(element: &str, classes: &'v str) -> Option<Cow<'v, str>> {
    let allowed = classes
        .split_ascii_whitespace()
        .filter(|class| {
            CLASSES.contains(class)
                || (element == "code" && class.starts_with(LANGUAGE_CLASS))
                || (element == "span" && is_highlight_class(class))
        })
        .collect::<Vec<_>>();
    (!allowed.is_empty()).then(|| allowed.join(" ").into())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::FILTERED_TAGS;
    use crate::Leave;
    use crate::files::fake::FakeFiles;
    use crate::render;

    fn html_of(markdown: &str) -> String {
        render(
            markdown.as_bytes(),
            Path::new("README.md"),
            "http://image.localhost",
            FakeFiles::default(),
            &Leave::new(),
        )
        .unwrap()
        .as_str()
        .to_owned()
    }

    #[test]
    fn details_with_summary_renders() {
        let html =
            html_of("<details open>\n<summary>More</summary>\n\nHidden text.\n\n</details>\n");
        assert!(html.contains(r#"<details open="">"#), "{html}");
        assert!(html.contains("<summary>More</summary>"), "{html}");
        assert!(html.contains(">Hidden text.</p>"), "{html}");
    }

    #[test]
    fn picture_keeps_its_sources_and_their_media() {
        let html = html_of(
            r#"<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://example.com/dark.png">
<img alt="Logo" src="https://example.com/light.png">
</picture>
"#,
        );
        assert!(
            html.contains(
                r#"<source media="(prefers-color-scheme: dark)" srcset="https://example.com/dark.png">"#
            ),
            "{html}"
        );
        assert!(
            html.contains(r#"<img alt="Logo" src="https://example.com/light.png">"#),
            "{html}"
        );
        assert!(html.contains("<picture>"), "{html}");
    }

    #[test]
    fn image_keeps_its_width_height_and_align() {
        let html = html_of(
            r#"<img src="https://example.com/logo.png" width="120" height="40" align="right">"#,
        );
        assert!(
            html.contains(
                r#"<img src="https://example.com/logo.png" width="120" height="40" align="right">"#
            ),
            "{html}"
        );
    }

    #[test]
    fn block_elements_keep_their_align() {
        let html = html_of(
            "<p align=\"center\">Centered</p>\n\n<div align=\"right\">\n\nRight\n\n</div>\n\n<h1 align=\"center\">Title</h1>\n",
        );
        assert!(html.contains(r#"<p align="center">Centered</p>"#), "{html}");
        assert!(html.contains(r#"<div align="right">"#), "{html}");
        assert!(html.contains(r#"<h1 align="center">Title</h1>"#), "{html}");
    }

    #[test]
    fn keys_subscripts_and_superscripts_render() {
        let html = html_of("Press <kbd>Ctrl</kbd>+<kbd>O</kbd>. H<sub>2</sub>O, x<sup>2</sup>.\n");
        assert!(
            html.contains("Press <kbd>Ctrl</kbd>+<kbd>O</kbd>. H<sub>2</sub>O, x<sup>2</sup>."),
            "{html}"
        );
    }

    #[test]
    fn scripts_are_removed_with_their_code() {
        let html = html_of(
            "<script>\nsteal()\n</script>\n\nText <script src=\"https://example.com/x.js\"></script>and <script>run()</script>more.\n",
        );
        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("steal"), "{html}");
        assert!(!html.contains("run()"), "{html}");
        assert!(html.contains(">Text and more.</p>"), "{html}");
    }

    #[test]
    fn styles_are_removed_with_their_rules() {
        let html = html_of(
            "<style>\nbody { display: none }\n</style>\n\n<p style=\"position: fixed\">Text</p>\n",
        );
        assert!(!html.contains("<style"), "{html}");
        assert!(!html.contains("display"), "{html}");
        assert!(!html.contains("style="), "{html}");
        assert!(html.contains("<p>Text</p>"), "{html}");
    }

    #[test]
    fn frames_and_objects_are_removed() {
        let html = html_of(
            r#"<iframe src="https://example.com"></iframe>
<frameset><frame src="https://example.com"></frameset>
<object data="movie.swf"></object>
<embed src="movie.swf">
"#,
        );
        for tag in ["<iframe", "<frame", "<object", "<embed"] {
            assert!(!html.contains(tag), "{tag}: {html}");
        }
        assert!(!html.contains("example.com"), "{html}");
        assert!(!html.contains("movie.swf"), "{html}");
    }

    #[test]
    fn event_handler_attributes_are_removed() {
        let html = html_of(
            r#"<img src="https://example.com/x.png" onerror="steal()"> <a href="https://example.com" onclick="steal()" onmouseover="steal()">link</a>"#,
        );
        assert!(!html.contains("steal"), "{html}");
        assert!(
            html.contains(r#"<img src="https://example.com/x.png">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<a href="https://example.com">link</a>"#),
            "{html}"
        );
    }

    #[test]
    fn javascript_urls_are_removed() {
        let html = html_of(
            "[Markdown](javascript:steal\\(\\)) <a href=\"JavaScript:steal()\">raw</a> <a href=\" &#x09;javascript:steal()\">spaced</a> <img src=\"javascript:steal()\">\n",
        );
        assert!(!html.contains("steal"), "{html}");
        assert!(html.contains("<a>Markdown</a>"), "{html}");
        assert!(html.contains("<a>raw</a>"), "{html}");
        assert!(html.contains("<a>spaced</a>"), "{html}");
    }

    #[test]
    fn tags_githubs_tagfilter_escapes_are_dropped() {
        for tag in FILTERED_TAGS {
            let html = html_of(&format!("Before <{tag}>inside</{tag}> after\n"));
            assert!(!html.contains(&format!("{tag}>")), "{tag}: {html}");
            if tag != "script" && tag != "style" {
                assert!(html.contains(">Before inside after</p>"), "{tag}: {html}");
            }
        }
    }

    #[test]
    fn filtered_tag_left_open_leaves_the_rest_of_the_document_alone() {
        for tag in FILTERED_TAGS {
            let html = html_of(&format!(
                "Text <{tag}> more\n\n# Next\n\nLast *word* & <b>bold</b>.\n"
            ));
            assert!(html.contains(">Next<a "), "{tag}: {html}");
            assert!(
                html.contains("Last <em>word</em> &amp; <b>bold</b>."),
                "{tag}: {html}"
            );
        }
    }

    #[test]
    fn filtered_tags_are_found_in_any_case_and_form() {
        for raw in [
            "<TextArea>",
            "<textarea\tname=x>",
            "<textarea/x>",
            "<PLAINTEXT>",
            "</xmp><xmp>",
        ] {
            let html = html_of(&format!("Text {raw} more\n\n# Next\n"));
            assert!(html.contains(">Next<a "), "{raw}: {html}");
        }
    }

    #[test]
    fn raw_html_loses_its_own_line_numbers_and_rendering_s_stay() {
        let html = html_of(
            "Text\n\n<div data-sourcepos=\"99:1-99:5\">Block</div>\n\nInline <span data-sourcepos=\"98:1-98:2\">span</span>.\n",
        );
        assert!(html.contains("<div>Block</div>"), "{html}");
        assert!(html.contains("<span>span</span>"), "{html}");
        assert_eq!(
            html.matches("data-sourcepos").count(),
            2,
            "only the two paragraphs': {html}"
        );
        assert!(
            html.contains(r#"<p data-sourcepos="1:1-1:4">Text</p>"#),
            "{html}"
        );
    }

    #[test]
    fn line_numbers_in_raw_html_are_found_in_any_case_and_form() {
        for raw in [
            r#"<div DATA-SourcePos="99:1-99:1">"#,
            "<div data-sourcepos=99:1-99:1>",
            "<div data-sourcepos='99:1-99:1'>",
            "<div id=\"a\" title=\"1 > 0\"\tdata-sourcepos = \"99:1-99:1\">",
            "<div title='say \"hi\"' data-sourcepos=\"99:1-99:1\">",
            "<div\ndata-sourcepos=\"99:1-99:1\">",
        ] {
            let html = html_of(&format!("{raw}\nBlock\n</div>\n"));
            assert!(!html.contains("99:1"), "{raw}: {html}");
            assert!(html.contains("Block"), "{raw}: {html}");
        }
    }

    /// A document whose raw HTML has `before` and `after` around a block with a line number of its
    /// own, which the HTML parser reads as a tag: the scan finds the line number only if it reads
    /// `before` as the parser does.
    fn html_with_a_line_number_after(before: &str, after: &str) -> String {
        html_of(&format!(
            "<div>\n{before}<div data-sourcepos=\"99:1-99:1\">Text</div>{after}\n</div>\n"
        ))
    }

    #[test]
    fn line_numbers_after_comments_and_declarations_are_dropped() {
        for (before, after) in [
            ("<!-- <a title=\" -->\n", "\n\""),
            ("<![CDATA[ <a title=\" ]]>\n", "\n\""),
            ("<?x title=\" ?>\n", "\n\""),
            ("<!x <a title=\" >\n", "\n\""),
            ("<!DOCTYPE x <a title=\" >\n", "\n\""),
            ("</1 <a title=\" >\n", "\n\""),
            ("</div title='<a x=\"'>\n", "\n\""),
        ] {
            let html = html_with_a_line_number_after(before, after);
            assert!(!html.contains("99:1"), "{before}: {html}");
            assert!(html.contains("Text"), "{before}: {html}");
        }
    }

    #[test]
    fn comment_ends_where_the_html_parser_ends_it() {
        for (before, after) in [
            // These end at once, before the `-->` that comes later.
            ("<!--> ", " -->"),
            ("<!---> ", " -->"),
            ("<!-- a --!> ", " -->"),
            ("<!-- a <!--> ", " -->"),
            // These don't end at their `>`, so the quote in them is no attribute's.
            ("<!--!> <a title=\" --> ", "\""),
            ("<!---!> <a title=\" --> ", "\""),
            ("<!-- a -!> <a title=\" --> ", "\""),
        ] {
            let html = html_with_a_line_number_after(before, after);
            assert!(!html.contains("99:1"), "{before}: {html}");
            assert!(html.contains("Text"), "{before}: {html}");
        }
    }

    #[test]
    fn line_number_after_cdata_that_svg_reads_differently_is_dropped() {
        // In `svg` the CDATA ends at `]]>`, elsewhere at its first `>`.
        let html = html_with_a_line_number_after("<svg><![CDATA[ > <a title=\"]]>\n", "\n\"");
        assert!(!html.contains("99:1"), "{html}");
    }

    #[test]
    fn raw_html_ending_inside_a_tag_shows_the_tag_as_text() {
        let html = html_of("<div title='\n\n' data-sourcepos='999:1-999:1'\n");
        assert!(!html.contains(r#"data-sourcepos="999"#), "{html}");
        assert!(html.contains("&lt;div title='"), "{html}");
        assert!(html.contains(r#"<p data-sourcepos="3:1-3:30">"#), "{html}");
    }

    #[test]
    fn unfinished_tag_takes_no_line_number_from_the_paragraph_after_it() {
        let html = html_of("<div\n\nText\n");
        assert!(html.contains("&lt;div"), "{html}");
        assert!(
            html.contains(r#"<p data-sourcepos="3:1-3:4">Text</p>"#),
            "{html}"
        );
    }

    #[test]
    fn raw_html_ending_inside_a_comment_hides_no_line_number_after_it() {
        // The second comment starts in an inline tag that comrak reads whole and the parser
        // ends at its first `>`, since `\x0b` separates attributes for comrak only.
        for document in [
            "> <!--\n\nText <a title=\"--><b data-sourcepos='99:1-99:1'>\">x</a>\n",
            "Text <a\x0bb=\">\" c=\"<!--\" d=e> and <a title=\"--><b data-sourcepos='99:1-99:1'>\">x</a>\n",
        ] {
            let html = html_of(document);
            assert!(
                !html.contains(r#"data-sourcepos="99"#),
                "{document}: {html}"
            );
        }
    }

    #[test]
    fn raw_html_ending_inside_a_declaration_hides_no_line_number_after_it() {
        let html = html_of("<div>\n<?x\n\n<a title=\"?><b data-sourcepos='99:1-99:1'>\">\n");
        assert!(!html.contains(r#"data-sourcepos="99"#), "{html}");
    }

    #[test]
    fn text_that_names_the_line_number_attribute_is_kept() {
        let html = html_of(
            "<div title=\"data-sourcepos=1\">\nThe data-sourcepos=\"1:1\" attribute, 2 < 3 data-sourcepos.\n</div>\n",
        );
        assert!(html.contains(r#"title="data-sourcepos=1""#), "{html}");
        assert!(
            html.contains(r#"The data-sourcepos="1:1" attribute, 2 &lt; 3 data-sourcepos."#),
            "{html}"
        );
    }

    #[test]
    fn sanitizing_keeps_highlighting_classes_on_spans_only() {
        let html = html_of(
            "```rust\nfn main() {}\n```\n\n<span class=\"pl-k pl-nope notice\">a</span> <em class=\"pl-k\">b</em>\n",
        );
        assert!(html.contains(r#"<span class="pl-k">fn</span>"#), "{html}");
        assert!(html.contains(r#"<span class="pl-k">a</span>"#), "{html}");
        assert!(html.contains("<em>b</em>"), "{html}");
    }

    #[test]
    fn document_keeps_only_the_classes_rendering_writes() {
        let html = html_of(
            "<p class=\"notice\">Fake message</p>\n\n<div class=\"empty-window markdown-alert markdown-alert-tip\">Tip</div>\n\nSee <code class=\"language-rust highlight\">fn</code>.\n",
        );
        assert!(html.contains("<p>Fake message</p>"), "{html}");
        assert!(
            html.contains(r#"<div class="markdown-alert markdown-alert-tip">Tip</div>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<code class="language-rust">fn</code>"#),
            "{html}"
        );
    }

    #[test]
    fn checkboxes_come_only_from_task_lists() {
        let html = html_of(
            "<input type=\"checkbox\" checked> <INPUT type=\"text\" value=\"Password\">\n\n- [ ] Task\n",
        );
        assert_eq!(html.matches("<input").count(), 1, "{html}");
        assert!(
            html.contains(
                r#"<input type="checkbox" class="task-list-item-checkbox" disabled=""> Task"#
            ),
            "{html}"
        );
    }

    #[test]
    fn links_keep_relative_web_mail_and_local_targets() {
        for target in [
            "docs/guide.md",
            "#install",
            "https://example.com/",
            "http://example.com/",
            "mailto:lamp@example.com",
            "file:///C:/docs/guide.md",
            "C:/docs/guide.md",
        ] {
            let html = html_of(&format!("[Link]({target})\n"));
            assert!(
                html.contains(&format!(r#"href="{target}""#)),
                "{target}: {html}"
            );
        }
    }

    #[test]
    fn links_with_other_schemes_lose_their_target() {
        for target in [
            "data:text/html,steal",
            "vbscript:steal",
            "ms-settings:privacy",
            "search-ms:query=steal",
        ] {
            let html = html_of(&format!("[Link]({target})\n"));
            assert!(html.contains("<a>Link</a>"), "{target}: {html}");
        }
    }
}
