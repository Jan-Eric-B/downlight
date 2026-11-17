//! Highlighting: code blocks with a known language are colored as on GitHub (ticket 07).
//!
//! syntect parses the code with bat's grammars (two-face), and each scope maps to one of GitHub's
//! own `pl-*` classes, which github-markdown-css colors for light and dark. So the page runs no
//! highlighting code, and a switch between light and dark needs no new render.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Write};
use std::str::FromStr;
use std::sync::LazyLock;

use comrak::adapters::SyntaxHighlighterAdapter;
use comrak::html::{escape, write_opening_tag};
use syntect::highlighting::{
    Color, HighlightIterator, HighlightState, Highlighter, ScopeSelectors, StyleModifier, Theme,
    ThemeItem, ThemeSettings,
};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// GitHub's rules from scope to class, as in its Primer syntax themes (the source of
/// github-markdown-css's `pl-*` colors). The most specific selector wins, as in any TextMate
/// theme. `None` is the default text color, for scopes that undo a broader rule, and writes no
/// span.
const RULES: &[(&str, Option<&str>)] = &[
    (
        "comment, punctuation.definition.comment, string.comment",
        Some("pl-c"),
    ),
    (
        "constant, entity.name.constant, variable.other.constant, variable.other.enummember, \
         variable.language, support, meta.property-name",
        Some("pl-c1"),
    ),
    ("entity", Some("pl-e")),
    ("entity.name", Some("pl-en")),
    ("entity.name.tag, support.class.component", Some("pl-ent")),
    ("keyword, storage, storage.type", Some("pl-k")),
    (
        "storage.modifier.package, storage.modifier.import, storage.type.java",
        None,
    ),
    ("string", Some("pl-s")),
    ("punctuation.definition.string", Some("pl-pds")),
    ("string.regexp", Some("pl-sr")),
    ("string.regexp constant.character.escape", Some("pl-sre")),
    ("string punctuation.section.embedded source", None),
    ("variable", Some("pl-v")),
    ("variable.function", Some("pl-en")),
    // A shell command's name and options show plain, as on GitHub; its builtins are `support`.
    ("variable.function.shell, variable.parameter.option", None),
    (
        "variable.other, variable.parameter.function, meta.embedded.expression",
        None,
    ),
    (
        "invalid.broken, invalid.deprecated, invalid.unimplemented",
        Some("pl-bu"),
    ),
    ("invalid.illegal", Some("pl-ii")),
    (
        "constant.other.reference.link, string.other.link",
        Some("pl-corl"),
    ),
    ("markup.heading, markup.heading entity.name", Some("pl-mh")),
    ("markup.list", Some("pl-ml")),
    ("markup.italic", Some("pl-mi")),
    ("markup.bold", Some("pl-mb")),
    (
        "markup.deleted, meta.diff.header.from-file, punctuation.definition.deleted",
        Some("pl-md"),
    ),
    (
        "markup.inserted, meta.diff.header.to-file, punctuation.definition.inserted",
        Some("pl-mi1"),
    ),
    (
        "markup.changed, punctuation.definition.changed",
        Some("pl-mc"),
    ),
    ("markup.ignored, markup.untracked", Some("pl-mi2")),
    ("meta.diff.range", Some("pl-mdr")),
];

// A rule's class travels as its index plus one in a color's red channel (`class_color`), and 0
// stands for no class, so every index plus one has to fit in a `u8`.
const _: () = assert!(RULES.len() < 255);

/// bat's grammars, loaded on the first code block that names a language. Each grammar compiles
/// its patterns only when a block first uses it.
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

/// [`RULES`] as a syntect theme: a rule's foreground color is no color but its class, as the
/// index into `RULES` plus one in the red channel. The default foreground (0) is no class.
static THEME: LazyLock<Theme> = LazyLock::new(|| Theme {
    settings: ThemeSettings {
        foreground: Some(class_color(0)),
        ..ThemeSettings::default()
    },
    scopes: RULES
        .iter()
        .enumerate()
        .filter_map(|(index, (selector, class))| {
            let class_index = class.map_or(0, |_| index + 1);
            Some(ThemeItem {
                // Parsing fails only for a scope of more than eight parts, which no rule has.
                scope: ScopeSelectors::from_str(selector).ok()?,
                style: StyleModifier {
                    foreground: Some(class_color(class_index)),
                    ..StyleModifier::default()
                },
            })
        })
        .collect(),
    ..Theme::default()
});

fn class_color(index: usize) -> Color {
    Color {
        r: u8::try_from(index).unwrap_or(0),
        g: 0,
        b: 0,
        a: 0xff,
    }
}

/// The class a style from [`THEME`] stands for.
fn class_of(color: Color) -> Option<&'static str> {
    let index = usize::from(color.r).checked_sub(1)?;
    RULES.get(index).and_then(|(_, class)| *class)
}

/// Whether highlighting writes `class`, for sanitizing to keep it.
pub(crate) fn is_highlight_class(class: &str) -> bool {
    RULES
        .iter()
        .any(|(_, rule_class)| *rule_class == Some(class))
}

/// comrak's highlighter plugin, writing GitHub's classes. Unlike comrak's own syntect adapter, it
/// keeps comrak's attributes on `pre` and `code`: the line number and the language class.
pub(crate) struct GithubHighlighter;

impl SyntaxHighlighterAdapter for GithubHighlighter {
    fn write_highlighted(
        &self,
        output: &mut dyn Write,
        lang: Option<&str>,
        code: &str,
    ) -> fmt::Result {
        // comrak passes the info string's first word; GitHub ignores what follows a comma too.
        let token = lang
            .map(|lang| lang.split_once(',').map_or(lang, |(token, _)| token))
            .unwrap_or_default();
        // No language, or one bat doesn't know: plain code, as on GitHub, which doesn't guess.
        let syntax = (!token.is_empty())
            .then(|| SYNTAXES.find_syntax_by_token(token))
            .flatten();
        let highlighted = syntax.and_then(|syntax| highlighted(code, syntax));
        match highlighted {
            Some(html) => output.write_str(&html),
            None => escape(output, code),
        }
    }

    fn write_pre_tag(
        &self,
        output: &mut dyn Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        write_opening_tag(output, "pre", attributes)
    }

    fn write_code_tag(
        &self,
        output: &mut dyn Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        write_opening_tag(output, "code", attributes)
    }
}

/// `code` as HTML with a span for each run of text in one class, or None if its grammar can't
/// parse a line, so the block shows plain.
fn highlighted(code: &str, syntax: &SyntaxReference) -> Option<String> {
    let highlighter = Highlighter::new(&THEME);
    let mut parse_state = ParseState::new(syntax);
    let mut highlight_state = HighlightState::new(&highlighter, ScopeStack::new());
    let mut html = Spans::with_capacity(code.len() * 2);
    for line in LinesWithEndings::from(code) {
        let changes = parse_state.parse_line(line, &SYNTAXES).ok()?;
        for (style, text) in
            HighlightIterator::new(&mut highlight_state, &changes, line, &highlighter)
        {
            html.push(class_of(style.foreground), text);
        }
    }
    Some(html.finish())
}

/// HTML written run by run, with neighboring runs in the same class in one span.
struct Spans {
    html: String,
    open: Option<&'static str>,
}

impl Spans {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            html: String::with_capacity(capacity),
            open: None,
        }
    }

    fn push(&mut self, class: Option<&'static str>, text: &str) {
        if class != self.open {
            if self.open.is_some() {
                self.html.push_str("</span>");
            }
            if let Some(class) = class {
                self.html.push_str(r#"<span class=""#);
                self.html.push_str(class);
                self.html.push_str(r#"">"#);
            }
            self.open = class;
        }
        // Writing to a String never fails, so there is no error to pass on.
        let _ = escape(&mut self.html, text);
    }

    fn finish(mut self) -> String {
        if self.open.is_some() {
            self.html.push_str("</span>");
        }
        self.html
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{RULES, THEME};
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
    fn theme_has_a_scope_for_every_rule() {
        // A rule whose selector didn't parse would be left out of the theme without a word.
        assert_eq!(THEME.scopes.len(), RULES.len());
    }

    #[test]
    fn rust_code_block_is_highlighted_with_githubs_classes() {
        let html = html_of("```rust\nfn main() {}\n```\n");
        assert!(html.contains(r#"<span class="pl-k">fn</span>"#), "{html}");
        assert!(
            html.contains(r#"<span class="pl-en">main</span>"#),
            "{html}"
        );
    }

    #[test]
    fn typescript_and_toml_blocks_are_highlighted() {
        for (block, keyword) in [
            (
                "```ts\nconst a = 1;\n```\n",
                r#"<span class="pl-k">const</span>"#,
            ),
            (
                "```typescript\nconst a = 1;\n```\n",
                r#"<span class="pl-k">const</span>"#,
            ),
            (
                "```toml\n[package]\n```\n",
                r#"<span class="pl-en">package</span>"#,
            ),
        ] {
            let html = html_of(block);
            assert!(html.contains(keyword), "{block}: {html}");
        }
    }

    #[test]
    fn code_block_with_unknown_language_shows_as_plain_code() {
        let html = html_of("```no-such-language\nfn main() {}\n```\n");
        assert!(
            html.contains(r#"<code class="language-no-such-language">fn main() {}"#),
            "{html}"
        );
        assert!(!html.contains("<span"), "{html}");
    }

    #[test]
    fn code_block_without_language_shows_as_plain_code_even_if_it_looks_like_one() {
        let html = html_of("```\n#!/bin/sh\necho hi\n```\n\n    #!/bin/sh\n    echo hi\n");
        assert!(!html.contains("<span"), "{html}");
        assert_eq!(html.matches("#!/bin/sh\necho hi\n").count(), 2, "{html}");
    }

    #[test]
    fn front_matter_is_highlighted_as_yaml() {
        let html = html_of("---\ntitle: Notes\n---\n# Doc\n");
        assert!(html.contains(r#"<code class="language-yaml">"#), "{html}");
        assert!(
            html.contains(r#"<span class="pl-ent">title</span>"#),
            "{html}"
        );
    }

    #[test]
    fn highlighted_code_block_keeps_its_line_number_and_language_class() {
        let html = html_of("Text.\n\n```rust\nfn main() {}\n```\n");
        assert!(
            html.contains(r#"<pre data-sourcepos="3:1-5:3"><code class="language-rust">"#),
            "{html}"
        );
    }

    #[test]
    fn highlighted_code_keeps_its_exact_text() {
        let code = "fn main() {\r\n\tlet s = \"<b>&amp;\";\r\n\tif a < b && c > d {}\r\n}";
        let html = html_of(&format!("```rust\r\n{code}\r\n```\r\n"));
        // comrak ends the block's lines with `\n`, and its last line too.
        let expected = format!("{}\n", code.replace("\r\n", "\n"));
        assert_eq!(text_of_code(&html), expected, "{html}");
    }

    /// The text of the first `code` element, as the page's `textContent` gives it.
    fn text_of_code(html: &str) -> String {
        let start = html
            .find("<code")
            .and_then(|at| html[at..].find('>').map(|end| at + end + 1));
        let end = html.find("</code>");
        let (Some(start), Some(end)) = (start, end) else {
            return String::new();
        };
        let mut text = String::new();
        let mut in_tag = false;
        for c in html[start..end].chars() {
            match c {
                '<' => in_tag = true,
                '>' if in_tag => in_tag = false,
                _ if !in_tag => text.push(c),
                _ => {}
            }
        }
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
    }
}
