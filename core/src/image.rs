//! Images: where the page loads a document's images from. Local images load through the app's
//! image scheme, never from the file system directly, and images on network shares don't load at
//! all, so a document can't make Windows send the reader's credentials to a server (spec:
//! Render, Images).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use url::Url;

use crate::file_url::{self, Target};
use crate::files::{FileSystem, Leads, is_local, leads};

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
/// a file that isn't an image the webview shows, or one with a link along its path that leads off
/// this computer (ticket 20), or whose links can't be read. The page asks for whatever a document
/// names, so the scheme checks again what rendering checked, and asks `files` for the links that
/// rendering can't see.
pub fn local_image(url_path: &str, files: &impl FileSystem) -> Option<LocalImage> {
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
    match leads(&path, files) {
        Ok(Leads::ThisComputer) => Some(LocalImage { path, media_type }),
        Ok(Leads::Share { .. } | Leads::Elsewhere) | Err(_) => None,
    }
}

/// Where a document's images load from.
pub(crate) struct ImageSources<F> {
    /// The document's folder as a `file:` URL, against which relative images resolve.
    folder: Option<Url>,
    /// Where the page loads local images from, such as `http://image.localhost`.
    origin: String,
    /// The caller's lookups.
    files: F,
}

impl<F: FileSystem> ImageSources<F> {
    pub(crate) fn new(document: &Path, origin: &str, files: F) -> Self {
        Self {
            folder: file_url::folder_of(document),
            origin: origin.to_owned(),
            files,
        }
    }

    /// The URL the page loads the image at `src` from, or None when it must not load: remote
    /// `http(s)` and `data:` images load as they are, local ones through the image scheme, and
    /// anything else, such as an image on a network share, not at all.
    pub(crate) fn source(&self, src: &str) -> Option<String> {
        match file_url::classify(src) {
            Target::Url(url) if matches!(url.scheme(), "http" | "https" | "data") => {
                Some(src.to_owned())
            }
            Target::File => self.local_source(src),
            Target::Url(_) | Target::Unusable => None,
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
    /// names a file on a network share. The document's query means nothing to a file and is
    /// dropped; in its place the URL names when the file was last saved, which the image scheme
    /// ignores (ticket 32). The fragment stays, for GitHub's `#gh-dark-mode-only` and the like.
    fn local_source(&self, src: &str) -> Option<String> {
        let file = file_url::resolve(self.folder.as_ref(), src)?;
        // Only a `file:` URL has a path. One with a host, from `//server/share` or
        // `\\server\share`, gives a network share's path, which isn't local.
        let path = file.to_file_path().ok()?;
        if !is_local(&path) {
            return None;
        }
        let query = self
            .modified(&path)
            .map(|nanos| format!("?modified={nanos}"))
            .unwrap_or_default();
        let fragment = file
            .fragment()
            .map(|fragment| format!("#{fragment}"))
            .unwrap_or_default();
        Some(format!("{}{}{query}{fragment}", self.origin, file.path()))
    }

    /// When the image's file at `path` was last saved, in nanoseconds since 1970, or None when
    /// that can't be told. A page that sees the same URL again shows the image it loaded before,
    /// whatever the file holds now, so the time is what makes a changed file load anew. The
    /// lookup follows links, so it is asked only once the path is known to stay on this computer
    /// (ticket 20); one that leads elsewhere keeps a URL without a time, which the image scheme
    /// refuses anyway.
    fn modified(&self, path: &Path) -> Option<u128> {
        if leads(path, &self.files).ok()? != Leads::ThisComputer {
            return None;
        }
        let modified = self.files.modified(path).ok()??;
        Some(
            modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .ok()?
                .as_nanos(),
        )
    }
}

// Windows paths: v1 ships for Windows only, and drive letters and network shares are Windows'.
// Two attributes, not `all(test, windows)`: Clippy's allows for tests (clippy.toml) count a
// module as tests only by a `cfg(test)` of its own.
#[cfg(test)]
#[cfg(windows)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, SystemTime};

    use super::local_image;
    use crate::Leave;
    use crate::files::fake::FakeFiles;
    use crate::render;

    const ORIGIN: &str = "http://image.localhost";

    /// The HTML of `markdown` in a document at `C:\docs\guide\README.md`, whose images' files
    /// have no modified time to tell.
    fn html_of(markdown: &str) -> String {
        html_with(markdown, FakeFiles::default())
    }

    /// The HTML of `markdown` in a document at `C:\docs\guide\README.md`, looking at `files`.
    fn html_with(markdown: &str, files: FakeFiles) -> String {
        render(
            markdown.as_bytes(),
            Path::new(r"C:\docs\guide\README.md"),
            ORIGIN,
            files,
            &Leave::new(),
        )
        .unwrap()
        .as_str()
        .to_owned()
    }

    /// 2023-11-14 22:13:20 UTC and 100 ns, written in the URL in nanoseconds since 1970.
    fn saved_at(extra_seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000 + extra_seconds, 100)
    }

    #[test]
    fn local_image_url_names_the_time_its_file_was_last_saved() {
        let files = FakeFiles::default().modified_at(r"C:\docs\guide\img\logo.png", saved_at(0));
        let html = html_with("![Logo](img/logo.png)\n", files);
        assert!(
            html.contains(
                r#"<img src="http://image.localhost/C:/docs/guide/img/logo.png?modified=1700000000000000100" alt="Logo">"#
            ),
            "{html}"
        );
    }

    #[test]
    fn local_image_url_stays_the_same_until_its_file_is_saved_again() {
        let render_at = |time| {
            let files = FakeFiles::default().modified_at(r"C:\docs\guide\img\logo.png", time);
            html_with("![Logo](img/logo.png)\n", files)
        };
        let first = render_at(saved_at(0));
        assert_eq!(render_at(saved_at(0)), first);
        let saved_again = render_at(saved_at(5));
        assert!(
            saved_again.contains(
                r#"src="http://image.localhost/C:/docs/guide/img/logo.png?modified=1700000005000000100""#
            ),
            "{saved_again}"
        );
    }

    #[test]
    fn image_whose_file_is_missing_loads_without_a_time() {
        let html = html_of("![Logo](img/missing.png)\n");
        assert!(
            html.contains(r#"src="http://image.localhost/C:/docs/guide/img/missing.png""#),
            "{html}"
        );
    }

    #[test]
    fn saved_time_comes_before_the_fragment_and_in_srcset_too() {
        let files = FakeFiles::default()
            .modified_at(r"C:\docs\logo.png", saved_at(0))
            .modified_at(r"C:\docs\guide\img\dark.png", saved_at(1));
        let html = html_with(
            r#"![Logo](../logo.png?raw=true#gh-dark-mode-only)

<picture><source srcset="img/dark.png 2x"><img alt="Logo" src="img/light.png"></picture>
"#,
            files,
        );
        assert!(
            html.contains(
                r#"src="http://image.localhost/C:/docs/logo.png?modified=1700000000000000100#gh-dark-mode-only""#
            ),
            "{html}"
        );
        assert!(
            html.contains(
                r#"srcset="http://image.localhost/C:/docs/guide/img/dark.png?modified=1700000001000000100 2x""#
            ),
            "{html}"
        );
    }

    #[test]
    fn image_url_with_its_saved_time_leads_back_to_its_file() {
        let files = FakeFiles::default().modified_at(r"C:\docs\guide\img\logo.png", saved_at(0));
        let html = html_with("![Logo](img/logo.png)\n", files);
        let src = html
            .split('"')
            .find(|part| part.starts_with(ORIGIN))
            .unwrap_or_default();
        // The image scheme looks only at the URL's path, as `app/src/image.rs` takes it.
        let url = url::Url::parse(src).unwrap();
        assert_eq!(url.query(), Some("modified=1700000000000000100"));
        let image = local_image(url.path(), &FakeFiles::default()).unwrap();
        assert_eq!(image.path(), Path::new(r"C:\docs\guide\img\logo.png"));
    }

    #[test]
    fn image_whose_path_links_off_this_computer_is_not_asked_when_it_was_saved() {
        // The fake would tell the time, so a URL that names one would mean the lookup followed the
        // link off this computer (ticket 20).
        for (link, target) in [
            (r"C:\docs\guide\img", r"\\server\share\pics"),
            (r"C:\docs\guide\img\logo.png", r"\\server\share\logo.png"),
            (
                r"C:\docs",
                r"\\?\Volume{0b1e3f6c-0000-0000-0000-100000000000}\docs",
            ),
        ] {
            let files = FakeFiles::default()
                .with_link(link, target)
                .modified_at(r"C:\docs\guide\img\logo.png", saved_at(0));
            let html = html_with("![Logo](img/logo.png)\n", files);
            assert!(
                html.contains(r#"src="http://image.localhost/C:/docs/guide/img/logo.png""#),
                "{link}: {html}"
            );
        }
    }

    #[test]
    fn image_whose_links_cannot_be_read_is_not_asked_when_it_was_saved() {
        let files = FakeFiles::default().modified_at(r"C:\docs\unreadable\logo.png", saved_at(0));
        let html = html_with("![Logo](../unreadable/logo.png)\n", files);
        assert!(
            html.contains(r#"src="http://image.localhost/C:/docs/unreadable/logo.png""#),
            "{html}"
        );
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
        let image = local_image(url_path, &FakeFiles::default()).unwrap();
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
            assert!(
                local_image(url_path, &FakeFiles::default()).is_none(),
                "{url_path}"
            );
        }
    }

    #[test]
    fn image_scheme_refuses_an_image_whose_path_links_to_a_share_without_looking_there() {
        for (link, target) in [
            (r"C:\docs\logo.png", r"\\server\share\x.png"),
            (r"C:\docs", r"\\server\share\docs"),
        ] {
            let files = FakeFiles::default().with_link(link, target);
            assert!(local_image("/C:/docs/logo.png", &files).is_none(), "{link}");
            assert!(!files.touched_a_share(), "{link}: {:?}", files.asked);
        }
    }

    #[test]
    fn image_scheme_serves_an_image_whose_path_links_to_this_computer() {
        let files = FakeFiles::default().with_link(r"C:\docs\logo.png", r"D:\art\logo.png");
        let image = local_image("/C:/docs/logo.png", &files).unwrap();
        // The file is read through the link, as the document names it.
        assert_eq!(image.path(), Path::new(r"C:\docs\logo.png"));
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
            let image = local_image(&format!("/C:/pics/{name}"), &FakeFiles::default());
            assert_eq!(
                image.map(|image| image.media_type()),
                Some(media_type),
                "{name}"
            );
        }
    }
}
