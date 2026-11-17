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

/// The elements GitHub allows in a document, and the ones rendering writes besides: `section`
/// around the footnotes and `input` for task list checkboxes.
const TAGS: &[&str] = &[
    "a",
    "abbr",
    "b",
    "bdo",
    "blockquote",
    "br",
    "caption",
    "cite",
    "code",
    "dd",
    "del",
    "details",
    "dfn",
    "div",
    "dl",
    "dt",
    "em",
    "figcaption",
    "figure",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "i",
    "img",
    "input",
    "ins",
    "kbd",
    "li",
    "mark",
    "ol",
    "p",
    "picture",
    "pre",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "section",
    "small",
    "source",
    "span",
    "strike",
    "strong",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "time",
    "tr",
    "tt",
    "ul",
    "var",
    "wbr",
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
/// new name like any unknown element, so nothing after it changes, and sanitizing drops it.
const DROPPED_PREFIX: &str = "dropped-";

/// The renamed tags whose content goes with them, as ammonia does for `script` and `style`.
const DROPPED_WITH_CONTENT: [&str; 2] = ["dropped-script", "dropped-style"];

/// Renames the filtered and the rendering-only tags in the document's raw HTML, the only place
/// they can come from: comrak escapes `<` everywhere else.
pub(crate) fn drop_filtered_tags<'a>(root: Node<'a>) {
    for node in root.descendants() {
        let mut data = node.data_mut();
        let raw = match &mut data.value {
            NodeValue::HtmlBlock(block) => &mut block.literal,
            NodeValue::HtmlInline(raw) => raw,
            _ => continue,
        };
        if let Some(renamed) = rename_filtered_tags(raw) {
            *raw = renamed;
        }
    }
}

/// `raw` with the name of each filtered or rendering-only tag prefixed, or None when it has none.
fn rename_filtered_tags(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut renamed: Option<String> = None;
    let mut copied = 0;
    for (at, _) in raw.match_indices('<') {
        let is_end_tag = bytes.get(at + 1) == Some(&b'/');
        let name_start = at + 1 + usize::from(is_end_tag);
        let mut dropped = FILTERED_TAGS.iter().chain(&RENDERING_ONLY_TAGS);
        if !dropped.any(|tag| names_tag(bytes, name_start, tag)) {
            continue;
        }
        let out =
            renamed.get_or_insert_with(|| String::with_capacity(raw.len() + DROPPED_PREFIX.len()));
        // `<` and `/` are ASCII, so `name_start` is a char boundary.
        out.push_str(&raw[copied..name_start]);
        out.push_str(DROPPED_PREFIX);
        copied = name_start;
    }
    renamed.map(|mut out| {
        out.push_str(&raw[copied..]);
        out
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
            .is_none_or(|next| matches!(next, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'/' | b'>'))
}

/// Sanitizes the HTML that comrak rendered, with its images loading from `images`. Run
/// [`drop_filtered_tags`] on the document's tree before rendering it.
pub(crate) fn sanitize<F: FileSystem + Send + 'static>(
    html: &str,
    images: ImageSources<F>,
) -> String {
    let tag_attributes = TAG_ATTRIBUTES
        .iter()
        .map(|(tag, attributes)| (*tag, attributes.iter().copied().collect()))
        .collect::<HashMap<_, HashSet<_>>>();
    Builder::empty()
        .tags(TAGS.iter().copied().collect())
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
    use crate::files::fake::FakeFiles;
    use crate::render;

    fn html_of(markdown: &str) -> String {
        render(
            markdown.as_bytes(),
            Path::new("README.md"),
            "http://image.localhost",
            FakeFiles::default(),
        )
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
