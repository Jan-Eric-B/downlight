//! Images: where the page loads a document's images from. Local images load through the app's
//! image scheme, never from the file system directly, and images on network shares don't load at
//! all, so a document can't make Windows send the reader's credentials to a server (spec:
//! Render, Images).

use std::path::{Component, Path, PathBuf, Prefix};

use url::Url;

/// The image formats the webview shows, by extension, with their media types.
const MEDIA_TYPES: [(&str, &str); 10] = [
    ("apng", "image/apng"),
    ("avif", "image/avif"),
    ("bmp", "image/bmp"),
    ("gif", "image/gif"),
    ("ico", "image/x-icon"),
    ("jpeg", "image/jpeg"),
    ("jpg", "image/jpeg"),
    ("png", "image/png"),
    ("svg", "image/svg+xml"),
    ("webp", "image/webp"),
];

/// A local image file that the app's image scheme serves, as [`local_image`] finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalImage {
    path: PathBuf,
    media_type: &'static str,
}

impl LocalImage {
    /// The image's file, a whole path on this computer.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The image's media type, such as `image/png`, by its extension.
    pub fn media_type(&self) -> &'static str {
        self.media_type
    }
}

/// The local image that a path of the image scheme's URLs names, as rendering writes them
/// (`/C:/docs/logo.png`), or None for anything else: a file on a network share, a relative path,
/// or a file that isn't an image the webview shows. The page asks for whatever a document names,
/// so the scheme checks again what rendering checked.
pub fn local_image(url_path: &str) -> Option<LocalImage> {
    let path = Url::parse(&format!("file://{url_path}"))
        .ok()?
        .to_file_path()
        .ok()?;
    if !is_local(&path) {
        return None;
    }
    let extension = path.extension()?.to_str()?;
    let (_, media_type) = MEDIA_TYPES
        .iter()
        .find(|(known, _)| extension.eq_ignore_ascii_case(known))?;
    Some(LocalImage { path, media_type })
}

/// Where a document's images load from.
pub(crate) struct ImageSources {
    /// The document's folder as a `file:` URL, against which relative images resolve.
    folder: Option<Url>,
    /// Where the page loads local images from, such as `http://image.localhost`.
    origin: String,
}

impl ImageSources {
    pub(crate) fn new(document: &Path, origin: &str) -> Self {
        let folder = document
            .parent()
            .and_then(|folder| Url::from_directory_path(folder).ok());
        Self {
            folder,
            origin: origin.to_owned(),
        }
    }

    /// The URL the page loads the image at `src` from, or None when it must not load: remote
    /// `http(s)` and `data:` images load as they are, local ones through the image scheme, and
    /// anything else, such as an image on a network share, not at all.
    pub(crate) fn source(&self, src: &str) -> Option<String> {
        match Url::parse(src) {
            Ok(url) if matches!(url.scheme(), "http" | "https" | "data") => Some(src.to_owned()),
            Ok(url) if url.scheme() != "file" && !is_drive(url.scheme()) => None,
            Ok(_) | Err(url::ParseError::RelativeUrlWithoutBase) => self.local_source(src),
            Err(_) => None,
        }
    }

    /// `srcset` with each image's URL as [`source`](Self::source) gives it, leaving out the images
    /// that must not load, or None when none is left. Each candidate is a URL, which runs to the
    /// next whitespace, and its descriptors (`2x`, `300w`) up to the next comma, as the HTML spec
    /// reads them; so a `data:` URL keeps its comma.
    pub(crate) fn srcset(&self, srcset: &str) -> Option<String> {
        let mut candidates = Vec::new();
        let mut rest = srcset;
        loop {
            rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
            if rest.is_empty() {
                break;
            }
            let url_end = rest
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(rest.len());
            let (url, after_url) = rest.split_at(url_end);
            let (url, descriptors) = if url.ends_with(',') {
                rest = after_url;
                (url.trim_end_matches(','), "")
            } else {
                let end = after_url.find(',').unwrap_or(after_url.len());
                rest = &after_url[end..];
                (url, after_url[..end].trim())
            };
            if let Some(source) = self.source(url) {
                candidates.push(if descriptors.is_empty() {
                    source
                } else {
                    format!("{source} {descriptors}")
                });
            }
        }
        (!candidates.is_empty()).then(|| candidates.join(", "))
    }

    /// The image scheme's URL for `src`, a relative or absolute path or a `file:` URL, unless it
    /// names a file on a network share. A query means nothing to a file and is dropped; the
    /// fragment stays, for GitHub's `#gh-dark-mode-only` and the like.
    fn local_source(&self, src: &str) -> Option<String> {
        // comrak writes a backslash in a Markdown link as %5C, and Windows paths separate
        // folders with either slash.
        let src = src.replace("%5C", "/").replace("%5c", "/");
        let file = if is_drive_path(&src) {
            Url::parse(&format!("file:///{src}"))
        } else {
            match &self.folder {
                Some(folder) => folder.join(&src),
                None => Url::parse(&src),
            }
        }
        .ok()?;
        // Only a `file:` URL has a path. One with a host, from `//server/share` or
        // `\\server\share`, gives a network share's path, which isn't local.
        if !is_local(&file.to_file_path().ok()?) {
            return None;
        }
        let fragment = file
            .fragment()
            .map(|fragment| format!("#{fragment}"))
            .unwrap_or_default();
        Some(format!("{}{}{fragment}", self.origin, file.path()))
    }
}

/// Whether a URL's scheme is really a drive letter, as the URL parser reads `C:\pics\logo.png`.
pub(crate) fn is_drive(scheme: &str) -> bool {
    scheme.len() == 1 && scheme.bytes().all(|letter| letter.is_ascii_alphabetic())
}

/// Whether `src` starts with a drive letter and a slash, as `C:/pics` or `C:\pics` do.
fn is_drive_path(src: &str) -> bool {
    matches!(src.as_bytes(), [letter, b':', b'/' | b'\\', ..] if letter.is_ascii_alphabetic())
}

/// Whether `path` is a whole path on this computer: on Windows, one on a drive, never a network
/// share or a device.
fn is_local(path: &Path) -> bool {
    path.is_absolute()
        && match path.components().next() {
            Some(Component::Prefix(prefix)) => {
                matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
            }
            // Only Windows has prefixes.
            _ => true,
        }
}

// Windows paths: v1 ships for Windows only, and drive letters and network shares are Windows'.
#[cfg(all(test, windows))]
mod tests {
    use std::path::Path;

    use super::local_image;
    use crate::render;

    const ORIGIN: &str = "http://image.localhost";

    /// The HTML of `markdown` in a document at `C:\docs\guide\README.md`.
    fn html_of(markdown: &str) -> String {
        render(
            markdown.as_bytes(),
            Path::new(r"C:\docs\guide\README.md"),
            ORIGIN,
        )
        .as_str()
        .to_owned()
    }

    #[test]
    fn relative_image_loads_from_the_documents_folder_through_the_image_scheme() {
        let html = html_of("![Logo](img/logo.png)\n");
        assert!(
            html.contains(
                r#"<img src="http://image.localhost/C:/docs/guide/img/logo.png" alt="Logo">"#
            ),
            "{html}"
        );
    }

    #[test]
    fn relative_image_keeps_its_fragment_and_drops_its_query() {
        let html = html_of("![Logo](../logo.png?raw=true#gh-dark-mode-only)\n");
        assert!(
            html.contains(r#"src="http://image.localhost/C:/docs/logo.png#gh-dark-mode-only""#),
            "{html}"
        );
    }

    #[test]
    fn image_names_with_spaces_and_umlauts_load() {
        let html = html_of(
            "![A](<my logo ä.png>) <img src=\"my logo ä.png\"> ![B](my%20logo%20%C3%A4.png)\n",
        );
        assert_eq!(
            html.matches(r#"src="http://image.localhost/C:/docs/guide/my%20logo%20%C3%A4.png""#)
                .count(),
            3,
            "{html}"
        );
    }

    #[test]
    fn absolute_local_paths_load_through_the_image_scheme() {
        for src in [
            r"C:\pics\logo.png",
            "C:/pics/logo.png",
            "file:///C:/pics/logo.png",
            "file://localhost/C:/pics/logo.png",
            "/pics/logo.png",
        ] {
            let expected = r#"src="http://image.localhost/C:/pics/logo.png""#;
            let markdown = html_of(&format!("![Logo](<{src}>)\n"));
            assert!(markdown.contains(expected), "{src}: {markdown}");
            let raw = html_of(&format!("<img src=\"{src}\">\n"));
            assert!(raw.contains(expected), "{src}: {raw}");
        }
    }

    #[test]
    fn remote_images_load_as_they_are() {
        for src in [
            "https://img.shields.io/badge/build-passing-green?style=flat#badge",
            "http://example.com/logo.png",
        ] {
            let html = html_of(&format!("![Badge]({src}) <img src=\"{src}\">\n"));
            let expected = format!(r#"src="{}""#, src.replace('&', "&amp;"));
            assert_eq!(html.matches(&expected).count(), 2, "{src}: {html}");
        }
    }

    #[test]
    fn data_url_images_load() {
        let src = "data:image/png;base64,iVBORw0KGgo=";
        let html = html_of(&format!("![Dot]({src}) <img src=\"{src}\">\n"));
        assert_eq!(
            html.matches(&format!(r#"src="{src}""#)).count(),
            2,
            "{html}"
        );
    }

    #[test]
    fn images_with_other_schemes_are_removed() {
        for src in [
            "javascript:steal()",
            "ftp://example.com/logo.png",
            "blob:https://example.com/1",
            "ms-appx:///logo.png",
        ] {
            let html = html_of(&format!("<img alt=\"Logo\" src=\"{src}\">\n"));
            assert!(html.contains(r#"<img alt="Logo">"#), "{src}: {html}");
        }
    }

    #[test]
    fn images_on_network_shares_are_removed() {
        for markdown in [
            r#"<img alt="Logo" src="\\server\share\logo.png">"#,
            r#"<img alt="Logo" src="//server/share/logo.png">"#,
            r#"<img alt="Logo" src="file://server/share/logo.png">"#,
            r#"<img alt="Logo" src="file:////server/share/logo.png">"#,
            r#"<img alt="Logo" src="\\?\UNC\server\share\logo.png">"#,
            r#"<img alt="Logo" src="%5C%5Cserver%5Cshare%5Clogo.png">"#,
            "![Logo](//server/share/logo.png)",
            // Markdown's escapes make this `\\server\share\logo.png`.
            r"![Logo](\\\\server\\share\\logo.png)",
            "![Logo](file://server/share/logo.png)",
            "![Logo](<file://server/share/my logo.png>)",
        ] {
            let html = html_of(markdown);
            assert!(!html.contains("server"), "{markdown}: {html}");
            assert!(html.contains(r#"<img alt="Logo">"#), "{markdown}: {html}");
        }
    }

    #[test]
    fn picture_sources_load_like_images() {
        let html = html_of(
            r#"<picture>
<source media="(prefers-color-scheme: dark)" srcset="img/dark.png, img/dark@2x.png 2x">
<source srcset="https://example.com/a.webp 1x,\\server\share\b.webp 2x">
<source srcset="data:image/png;base64,iVBORw0KGgo=">
<source srcset="\\server\share\c.webp">
<img alt="Logo" src="img/light.png">
</picture>
"#,
        );
        assert!(
            html.contains(r#"srcset="http://image.localhost/C:/docs/guide/img/dark.png, http://image.localhost/C:/docs/guide/img/dark@2x.png 2x""#),
            "{html}"
        );
        assert!(
            html.contains(r#"<source srcset="https://example.com/a.webp 1x">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<source srcset="data:image/png;base64,iVBORw0KGgo=">"#),
            "{html}"
        );
        assert!(html.contains("<source>"), "{html}");
        assert!(!html.contains("server"), "{html}");
    }

    #[test]
    fn backslashes_in_relative_paths_separate_folders() {
        let html = html_of(r"![Logo](..\img\logo.png)");
        assert!(
            html.contains(r#"src="http://image.localhost/C:/docs/img/logo.png""#),
            "{html}"
        );
    }

    #[test]
    fn image_url_leads_back_to_its_file() {
        let html = html_of("![Logo](<img/my logo ä.png>)\n");
        let url_path = "/C:/docs/guide/img/my%20logo%20%C3%A4.png";
        assert!(html.contains(&format!("{ORIGIN}{url_path}")), "{html}");
        let image = local_image(url_path).unwrap();
        assert_eq!(image.path(), Path::new(r"C:\docs\guide\img\my logo ä.png"));
        assert_eq!(image.media_type(), "image/png");
    }

    #[test]
    fn image_scheme_serves_only_local_image_files() {
        for url_path in [
            "//server/share/logo.png",
            "/%5C%5Cserver%5Cshare%5Clogo.png",
            "server/share/logo.png",
            "/logo.png",
            "/C:/docs/notes.txt",
            "/C:/docs/README.md",
            "/C:/docs/logo",
        ] {
            assert!(local_image(url_path).is_none(), "{url_path}");
        }
    }

    #[test]
    fn each_image_format_the_webview_shows_gets_its_media_type() {
        for (name, media_type) in [
            ("a.png", "image/png"),
            ("a.apng", "image/apng"),
            ("a.jpg", "image/jpeg"),
            ("a.JPEG", "image/jpeg"),
            ("a.gif", "image/gif"),
            ("a.webp", "image/webp"),
            ("a.avif", "image/avif"),
            ("a.svg", "image/svg+xml"),
            ("a.bmp", "image/bmp"),
            ("a.ico", "image/x-icon"),
        ] {
            let image = local_image(&format!("/C:/pics/{name}"));
            assert_eq!(
                image.map(|image| image.media_type()),
                Some(media_type),
                "{name}"
            );
        }
    }
}
