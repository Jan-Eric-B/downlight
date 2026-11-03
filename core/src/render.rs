use std::path::Path;

use comrak::{Options, markdown_to_html};

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

/// Renders a document's file to HTML.
///
/// Markdown is the only format in v1; the path will pick the format by extension and resolve
/// relative images against the document's folder. Raw HTML in the document is left out of the
/// result until ticket 06 adds sanitizing.
pub fn render(
    bytes: &[u8],
    #[expect(
        unused_variables,
        reason = "ticket 06 resolves relative images against the document's folder"
    )]
    path: &Path,
) -> RenderedDocument {
    // Ticket 05 replaces this with decoding by byte order mark, UTF-8, then Windows-1252.
    let text = String::from_utf8_lossy(bytes);
    RenderedDocument {
        html: markdown_to_html(&text, &Options::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown_as_html() {
        let rendered = render(b"# Title\n\nSome *text*.\n", Path::new("README.md"));
        let html = rendered.as_str();

        assert!(html.contains(">Title</h1>"), "{html}");
        assert!(html.contains("<em>text</em>"), "{html}");
    }

    #[test]
    fn script_in_document_never_reaches_the_html() {
        let rendered = render(
            b"Before\n\n<script>alert('x')</script>\n\nAfter <script>alert('y')</script>\n",
            Path::new("README.md"),
        );
        let html = rendered.as_str();

        assert!(!html.contains("<script"), "{html}");
        assert!(html.contains("After"), "{html}");
    }
}
