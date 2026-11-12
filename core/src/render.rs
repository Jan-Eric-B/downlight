use std::path::Path;

use comrak::nodes::{Node, NodeCodeBlock, NodeValue};
use comrak::options::Plugins;
use comrak::{Arena, Options, format_html_with_plugins, parse_document};

use crate::decode::decode;
use crate::highlight::GithubHighlighter;
use crate::image::ImageSources;
use crate::sanitize::{drop_filtered_tags, sanitize};

/// A document rendered to HTML by [`render`].
///
/// Only `render` makes one, so HTML that reaches the page has always passed through the core's
/// sanitizing. It serializes as the plain HTML string the page shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedDocument {
    html: String,
}

impl RenderedDocument {
    /// The rendered HTML.
    pub fn as_str(&self) -> &str {
        &self.html
    }
}

impl serde::Serialize for RenderedDocument {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.html)
    }
}

/// Renders a document's file to HTML, the way GitHub shows it: GitHub Flavored Markdown,
/// footnotes, alerts, GitHub's heading anchors, emoji shortcodes, and front matter as a YAML code
/// block. Every block carries its line number in `data-sourcepos` (`line:column-line:column`).
///
/// The bytes are decoded first: a byte order mark decides the encoding (UTF-8, UTF-16 LE or BE),
/// otherwise valid UTF-8 is UTF-8, and anything else is Windows-1252.
///
/// The HTML is sanitized: it keeps the raw HTML GitHub allows and nothing that could act on its
/// own. `path` is the document's file, against whose folder relative images resolve, and
/// `image_origin` is where the page loads local images from: the app's image scheme, such as
/// `http://image.localhost`. Markdown is the only format in v1.
pub fn render(bytes: &[u8], path: &Path, image_origin: &str) -> RenderedDocument {
    let text = decode(bytes);
    let options = github_options();
    let arena = Arena::new();
    let root = parse_document(&arena, &text, &options);
    show_front_matter_as_yaml(root);
    keep_line_numbers_on_blocks_only(root);
    drop_filtered_tags(root);
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&GithubHighlighter);
    let mut html = String::new();
    // Writing to a String never fails, so there is no error to pass on.
    let _ = format_html_with_plugins(root, &options, &mut html, &plugins);
    let images = ImageSources::new(path, image_origin);
    RenderedDocument {
        html: sanitize(&html, images),
    }
}

const FRONT_MATTER_DELIMITER: &str = "---";

/// comrak's options for rendering as GitHub does. comrak 0.59 has no tagfilter (removed in 0.56);
/// sanitizing removes the tags it escaped.
fn github_options() -> Options<'static> {
    let mut options = Options::default();
    let extension = &mut options.extension;
    extension.table = true;
    extension.strikethrough = true;
    extension.autolink = true;
    extension.tasklist = true;
    extension.footnotes = true;
    extension.alerts = true;
    extension.shortcodes = true;
    // GitHub's anchors without its `user-content-` prefix, so that `#installation` finds its
    // heading.
    extension.header_id_prefix = Some(String::new());
    extension.front_matter_delimiter = Some(FRONT_MATTER_DELIMITER.to_owned());
    let render = &mut options.render;
    // Raw HTML stays in, for sanitizing to keep what GitHub allows.
    render.r#unsafe = true;
    render.sourcepos = true;
    // github-markdown-css styles task lists by these classes.
    render.tasklist_classes = true;
    options
}

/// comrak leaves front matter out of the HTML; the spec shows it as a YAML code block at the top.
/// The block keeps the front matter's source position, so it carries line 1 like any block.
fn show_front_matter_as_yaml<'a>(root: Node<'a>) {
    let Some(first) = root.first_child() else {
        return;
    };
    let mut data = first.data_mut();
    let NodeValue::FrontMatter(front_matter) = &data.value else {
        return;
    };
    let yaml = yaml_of(front_matter);
    data.value = NodeValue::CodeBlock(Box::new(NodeCodeBlock {
        fenced: true,
        fence_char: b'`',
        fence_length: 3,
        fence_offset: 0,
        info: "yaml".to_owned(),
        literal: yaml,
        closed: true,
    }));
}

/// comrak writes `data-sourcepos` on inline elements too, with no option to stop it. Only blocks
/// carry line numbers (glossary: Line number), and the inline ones would grow a 1 MB document's
/// HTML for nothing, so they are cleared: comrak writes none for a position on line 0.
fn keep_line_numbers_on_blocks_only<'a>(root: Node<'a>) {
    for node in root.descendants() {
        let mut data = node.data_mut();
        if !data.value.block() {
            data.sourcepos = (0, 0, 0, 0).into();
        }
    }
}

/// The YAML between the front matter's delimiter lines, with `\n` line endings.
fn yaml_of(front_matter: &str) -> String {
    let mut lines = front_matter.lines().skip(1).collect::<Vec<_>>();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines
        .last()
        .is_some_and(|line| line.trim_end() == FRONT_MATTER_DELIMITER)
    {
        lines.pop();
    }
    lines.iter().map(|line| format!("{line}\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE_ORIGIN: &str = "http://image.localhost";

    #[test]
    fn renders_markdown_as_html() {
        let rendered = render(
            b"# Title\n\nSome *text*.\n",
            Path::new("README.md"),
            IMAGE_ORIGIN,
        );
        let html = rendered.as_str();

        assert!(html.contains(">Title<a "), "{html}");
        assert!(html.contains("<em>text</em>"), "{html}");
    }

    #[test]
    fn script_in_document_never_reaches_the_html() {
        let rendered = render(
            b"Before\n\n<script>alert('x')</script>\n\nAfter <script>alert('y')</script>\n",
            Path::new("README.md"),
            IMAGE_ORIGIN,
        );
        let html = rendered.as_str();

        assert!(!html.contains("<script"), "{html}");
        assert!(html.contains("After"), "{html}");
    }

    fn html_of_bytes(bytes: &[u8]) -> String {
        render(bytes, Path::new("README.md"), IMAGE_ORIGIN)
            .as_str()
            .to_owned()
    }

    #[test]
    fn file_written_by_windows_powershell_5_1_out_file_shows_correctly() {
        // `'# Größe', '', 'Café' | Out-File ps.md` in Windows PowerShell 5.1.26100: UTF-16 LE
        // with a byte order mark and CRLF line endings.
        let bytes = [
            0xff, 0xfe, 0x23, 0x00, 0x20, 0x00, 0x47, 0x00, 0x72, 0x00, 0xf6, 0x00, 0xdf, 0x00,
            0x65, 0x00, 0x0d, 0x00, 0x0a, 0x00, 0x0d, 0x00, 0x0a, 0x00, 0x43, 0x00, 0x61, 0x00,
            0x66, 0x00, 0xe9, 0x00, 0x0d, 0x00, 0x0a, 0x00,
        ];
        let html = html_of_bytes(&bytes);
        // A mark that showed would come before the `#`, and the line would be no heading.
        assert!(html.starts_with("<h1 "), "{html}");
        assert!(html.contains(">Größe<a "), "{html}");
        assert!(html.contains(">Café</p>"), "{html}");
    }

    #[test]
    fn utf_8_byte_order_mark_decides_and_does_not_show() {
        let html = html_of_bytes(b"\xef\xbb\xbf# Gr\xc3\xb6\xc3\x9fe\n");
        assert!(html.starts_with("<h1 "), "{html}");
        assert!(html.contains(">Größe<a "), "{html}");
    }

    #[test]
    fn utf_16_be_byte_order_mark_decides_and_does_not_show() {
        let html = html_of_bytes(&[
            0xfe, 0xff, 0x00, 0x23, 0x00, 0x20, 0x00, 0x47, 0x00, 0x72, 0x00, 0xf6, 0x00, 0xdf,
            0x00, 0x65, 0x00, 0x0a,
        ]);
        assert!(html.starts_with("<h1 "), "{html}");
        assert!(html.contains(">Größe<a "), "{html}");
    }

    #[test]
    fn byte_order_mark_decides_even_when_the_rest_is_not_in_its_encoding() {
        // Windows-1252 bytes behind a UTF-8 mark: the mark decides, and nothing is retried.
        let html = html_of_bytes(b"\xef\xbb\xbfGr\xf6\xdfe\n");
        assert!(html.contains(">Gr\u{fffd}\u{fffd}e</p>"), "{html}");
    }

    #[test]
    fn valid_utf_8_without_a_byte_order_mark_is_read_as_utf_8() {
        // As Windows-1252 these bytes would read "GrÃ¶ÃŸe".
        let html = html_of_bytes(b"Gr\xc3\xb6\xc3\x9fe\n");
        assert!(html.contains(">Größe</p>"), "{html}");
    }

    #[test]
    fn windows_1252_file_with_umlauts_shows_umlauts() {
        // "Größe: 5 €" in Windows-1252, which is no valid UTF-8.
        let html = html_of_bytes(b"Gr\xf6\xdfe: 5 \x80\n");
        assert!(html.contains(">Größe: 5 €</p>"), "{html}");
    }

    fn html_of(markdown: &str) -> String {
        render(markdown.as_bytes(), Path::new("README.md"), IMAGE_ORIGIN)
            .as_str()
            .to_owned()
    }

    /// The line each `<tag ...>` in the HTML starts at, by its `data-sourcepos`, in order. A tag
    /// without one gives None.
    fn start_lines(html: &str, tag: &str) -> Vec<Option<u32>> {
        let opening = format!("<{tag}");
        html.match_indices(&opening)
            .filter(|(at, _)| {
                // `<p` must not match `<pre`.
                html[at + opening.len()..].starts_with([' ', '>', '/'])
            })
            .map(|(at, _)| {
                let tag_text = &html[at..at + html[at..].find('>').unwrap_or(0)];
                let (_, sourcepos) = tag_text.split_once("data-sourcepos=\"")?;
                sourcepos.split(':').next()?.parse().ok()
            })
            .collect()
    }

    #[test]
    fn table_renders_as_a_table() {
        let html = html_of("| Option | Meaning |\n| --- | --- |\n| `--size` | Card size |\n");
        assert!(html.contains("<table"), "{html}");
        assert!(html.contains("<th"), "{html}");
        assert!(html.contains("<td"), "{html}");
    }

    #[test]
    fn task_list_items_render_as_checkboxes_with_githubs_classes() {
        let html = html_of("- [x] Done\n- [ ] Not yet\n");
        assert!(html.contains(r#"class="contains-task-list""#), "{html}");
        assert!(html.contains(r#"class="task-list-item""#), "{html}");
        assert!(
            html.contains(
                r#"<input type="checkbox" class="task-list-item-checkbox" checked="" disabled="">"#
            ),
            "{html}"
        );
        assert!(
            html.contains(r#"<input type="checkbox" class="task-list-item-checkbox" disabled="">"#),
            "{html}"
        );
    }

    #[test]
    fn strikethrough_renders_as_del() {
        let html = html_of("~~gone~~\n");
        assert!(html.contains("<del>gone</del>"), "{html}");
    }

    #[test]
    fn bare_url_becomes_a_link() {
        let html = html_of("See https://example.com/docs for more.\n");
        assert!(
            html.contains(r#"<a href="https://example.com/docs">https://example.com/docs</a>"#),
            "{html}"
        );
    }

    #[test]
    fn footnote_reference_links_to_its_note_and_back() {
        let html = html_of("Text.[^1]\n\n[^1]: The note.\n");
        assert!(html.contains(r##"href="#fn-1""##), "{html}");
        assert!(html.contains(r#"id="fnref-1""#), "{html}");
        assert!(html.contains(r#"id="fn-1""#), "{html}");
        assert!(html.contains(r##"href="#fnref-1""##), "{html}");
        assert!(html.contains("The note."), "{html}");
    }

    #[test]
    fn every_alert_kind_renders_as_a_callout_with_its_title() {
        for (marker, kind, title) in [
            ("NOTE", "note", "Note"),
            ("TIP", "tip", "Tip"),
            ("IMPORTANT", "important", "Important"),
            ("WARNING", "warning", "Warning"),
            ("CAUTION", "caution", "Caution"),
        ] {
            let html = html_of(&format!("> [!{marker}]\n> Take care.\n"));
            assert!(
                html.contains(&format!(r#"class="markdown-alert markdown-alert-{kind}""#)),
                "{marker}: {html}"
            );
            assert!(
                html.contains(&format!(r#"<p class="markdown-alert-title">{title}</p>"#)),
                "{marker}: {html}"
            );
        }
    }

    #[test]
    fn headings_get_githubs_anchors() {
        for (markdown, id) in [
            ("# Installation", "installation"),
            ("# What's new?", "whats-new"),
            ("# Über uns", "über-uns"),
            ("# Step 1: Build & run", "step-1-build--run"),
            ("# snake_case and kebab-case", "snake_case-and-kebab-case"),
        ] {
            let html = html_of(markdown);
            assert!(
                html.contains(&format!(r#"id="{id}""#)),
                "{markdown}: {html}"
            );
            assert!(
                html.contains(&format!(r##"href="#{id}""##)),
                "{markdown}: {html}"
            );
        }
    }

    #[test]
    fn repeated_headings_get_numbered_anchors() {
        let html = html_of("# Install\n\n## Install\n\n### Install\n");
        assert!(html.contains(r#"id="install""#), "{html}");
        assert!(html.contains(r#"id="install-1""#), "{html}");
        assert!(html.contains(r#"id="install-2""#), "{html}");
    }

    #[test]
    fn emoji_shortcode_renders_as_emoji() {
        let html = html_of("Released :tada:\n");
        assert!(html.contains("Released 🎉"), "{html}");
    }

    #[test]
    fn unknown_shortcode_stays_as_text() {
        let html = html_of("Not an emoji :no-such-emoji:\n");
        assert!(html.contains(":no-such-emoji:"), "{html}");
    }

    #[test]
    fn front_matter_shows_as_a_yaml_code_block_at_the_top() {
        let html = html_of("---\ntitle: Notes\ntags: [a, b]\n---\n# Doc\n");
        // Highlighted as YAML since ticket 07, so its text is split into spans.
        assert!(
            html.starts_with(
                "<pre data-sourcepos=\"1:1-4:3\"><code class=\"language-yaml\"><span class=\"pl-ent\">title</span>: "
            ),
            "{html}"
        );
        assert!(html.contains(">tags</span>: "), "{html}");
        assert!(!html.contains("---"), "{html}");
        assert_eq!(start_lines(&html, "h1"), [Some(5)], "{html}");
    }

    #[test]
    fn front_matter_with_crlf_line_endings_shows_as_yaml() {
        let html = html_of("---\r\ntitle: Notes\r\n---\r\n\r\n# Doc\r\n");
        assert!(
            html.contains(
                "<code class=\"language-yaml\"><span class=\"pl-ent\">title</span>: <span class=\"pl-s\">Notes</span>\n</code>"
            ),
            "{html}"
        );
        assert_eq!(start_lines(&html, "h1"), [Some(5)], "{html}");
    }

    #[test]
    fn front_matter_with_html_in_it_shows_as_text() {
        let html = html_of("---\nnote: <b>bold</b>\n---\n");
        assert!(html.contains("&lt;b&gt;bold&lt;/b&gt;"), "{html}");
        assert!(!html.contains("<b>"), "{html}");
    }

    #[test]
    fn thematic_break_at_the_top_is_not_front_matter() {
        let html = html_of("---\n\nText\n");
        assert!(!html.contains("language-yaml"), "{html}");
        assert!(html.contains("<hr"), "{html}");
    }

    #[test]
    fn inline_elements_carry_no_line_number() {
        let html = html_of("Some *text* with `code`, ~~a~~ and a [link](a.md).\n");
        assert_eq!(html.matches("data-sourcepos").count(), 1, "{html}");
        assert!(html.starts_with("<p data-sourcepos="), "{html}");
    }

    #[test]
    fn every_block_carries_the_line_it_starts_at() {
        let markdown = "\
# Title

Paragraph.

- one
- two

1. first

```sh
code
```

    indented

> quote

| A | B |
| --- | --- |
| 1 | 2 |

---

> [!NOTE]
> Noted.

## Section[^n]

[^n]: Footnote.
";
        let html = html_of(markdown);
        for (tag, lines) in [
            ("h1", vec![Some(1)]),
            ("ul", vec![Some(5)]),
            ("li", vec![Some(5), Some(6), Some(8), Some(29)]),
            // The footnotes' list is comrak's own, inside a section that carries the line.
            ("ol", vec![Some(8), None]),
            ("section", vec![Some(29)]),
            ("pre", vec![Some(10), Some(14)]),
            ("blockquote", vec![Some(16)]),
            ("table", vec![Some(18)]),
            ("tr", vec![Some(18), Some(20)]),
            ("hr", vec![Some(22)]),
            ("div", vec![Some(24)]),
            ("h2", vec![Some(27)]),
        ] {
            assert_eq!(start_lines(&html, tag), lines, "<{tag}>: {html}");
        }
        assert_eq!(
            start_lines(&html, "p").first().copied().flatten(),
            Some(3),
            "{html}"
        );
    }
}
