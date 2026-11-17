//! Link resolution (spec: Link resolution): what clicking a link in a document does. A link
//! target comes from the document and is untrusted, so each one gives one safe action, and only
//! `http`, `https` and `mailto` URLs ever reach the system.

use std::path::{Path, PathBuf};
use std::{fmt, io};

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use url::Url;

use crate::file_url::{self, Target};
use crate::files::{FileSystem, Leads, PathKind, is_on_share_of, leads};
use crate::format::is_markdown;

/// What following a link does (glossary: Link action).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkAction {
    /// Show the Markdown document at `path` in this window, at `heading` if the link names one,
    /// or else at its top.
    ShowDocument {
        path: PathBuf,
        heading: Option<String>,
    },
    /// Scroll the shown document to the heading with this id, or to its top without one.
    ScrollToHeading { heading: Option<String> },
    /// Show the file selected in the file manager, never opening or running it.
    RevealFile(PathBuf),
    /// Open the folder in the file manager.
    OpenFolder(PathBuf),
    /// Hand an `http`, `https` or `mailto` URL to the system's browser or mail app.
    OpenUrl(String),
    /// Do nothing, as for a link with any other scheme.
    Nothing,
}

/// Why a link can't be followed.
#[derive(Debug)]
pub enum LinkError {
    /// The link names a file or folder that doesn't exist.
    NotFound { path: PathBuf },
    /// Whether the link's file or folder exists can't be told, as for a network share that is
    /// offline or a folder the reader may not open.
    Unreachable { path: PathBuf, source: io::Error },
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(f, "{} doesn't exist", path.display()),
            Self::Unreachable { path, .. } => write!(f, "{} can't be reached", path.display()),
        }
    }
}

impl std::error::Error for LinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotFound { .. } => None,
            Self::Unreachable { source, .. } => Some(source),
        }
    }
}

/// The action for the link `target` (glossary: Link target) in `document`, the document's file.
///
/// The core reads no files, so `files` tells whether a path is a file or a folder, and what the
/// links along it point to; it is asked only about a link to a path. A path that leads to a
/// network share, itself or through a link, is looked at only if `ask_to_connect`, given the path
/// and the share's host, says yes, since looking connects to that host (ticket 20); a no, or a path
/// that leads anywhere else off this computer, does nothing. A share on the host whose share
/// `document`'s path names is looked at without asking: reading the document connected there
/// already.
pub fn resolve_link(
    document: &Path,
    target: &str,
    files: &impl FileSystem,
    ask_to_connect: impl FnOnce(&Path, &str) -> bool,
) -> Result<LinkAction, LinkError> {
    let target = target.trim();
    if let Some(fragment) = target.strip_prefix('#') {
        return Ok(LinkAction::ScrollToHeading {
            heading: heading(fragment),
        });
    }
    // Relative to the document's folder, these would name the folder rather than the document.
    if target.is_empty() || target.starts_with('?') {
        return Ok(LinkAction::Nothing);
    }
    let file = match file_url::classify(target) {
        Target::Url(url) if goes_to_the_system(&url) => {
            return Ok(LinkAction::OpenUrl(for_the_system(&url)));
        }
        Target::File => file_url::resolve(file_url::folder_of(document).as_ref(), target),
        Target::Url(_) | Target::Unusable => None,
    };
    let Some(file) = file else {
        return Ok(LinkAction::Nothing);
    };
    let Ok(path) = file.to_file_path() else {
        return Ok(LinkAction::Nothing);
    };
    let may_look = match leads(&path, files) {
        Ok(Leads::ThisComputer) => true,
        Ok(Leads::Share { host }) => {
            is_on_share_of(document, &host) || ask_to_connect(&path, &host)
        }
        Ok(Leads::Elsewhere) => false,
        Err(source) => return Err(LinkError::Unreachable { path, source }),
    };
    if !may_look {
        return Ok(LinkAction::Nothing);
    }
    let kind = match files.kind_of(&path) {
        Ok(kind) => kind,
        Err(source) => return Err(LinkError::Unreachable { path, source }),
    };
    match kind {
        None => Err(LinkError::NotFound { path }),
        Some(PathKind::Folder) => Ok(LinkAction::OpenFolder(path)),
        Some(PathKind::File) if is_markdown(&path) => Ok(LinkAction::ShowDocument {
            path,
            heading: file.fragment().and_then(heading),
        }),
        Some(PathKind::File) => Ok(LinkAction::RevealFile(path)),
    }
}

/// The schemes whose URLs go to the system's browser or mail app.
const SYSTEM_SCHEMES: [&str; 3] = ["http", "https", "mailto"];

/// Whether following a link to `url` hands it to the system's browser or mail app. Sanitizing
/// asks too, so it keeps exactly the links that do something when clicked.
pub(crate) fn goes_to_the_system(url: &Url) -> bool {
    SYSTEM_SCHEMES.contains(&url.scheme())
}

/// What the URL parser leaves unencoded in a `mailto:` URL and the system's handler would split
/// the URL at: a browser or mail app gets the URL on its command line, so a space or a quote in
/// it must not end the argument.
const ARGUMENT_BREAKS: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

/// `url` as the system gets it: as the URL parser writes it, which encodes what a web URL can't
/// hold, and with what it leaves in a `mailto:` URL encoded too.
fn for_the_system(url: &Url) -> String {
    utf8_percent_encode(url.as_str(), ARGUMENT_BREAKS).to_string()
}

/// A link's fragment as the heading id it names, or None for an empty one, which names the top
/// of the document. The URL parser percent-encodes a fragment, and a heading's id keeps its
/// letters as they are (`#über-uns`), so both spellings find it.
fn heading(fragment: &str) -> Option<String> {
    (!fragment.is_empty()).then(|| {
        percent_decode_str(fragment)
            .decode_utf8_lossy()
            .into_owned()
    })
}

// Windows paths: v1 ships for Windows only, and drive letters and network shares are Windows'.
#[cfg(all(test, windows))]
mod tests {
    use std::io;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::files::fake::FakeFiles;

    const DOCUMENT: &str = r"C:\docs\guide\README.md";

    /// The reader's answer when a link asks to connect to a share: yes, as the tests that resolve
    /// links on shares need.
    fn yes(_: &Path, _: &str) -> bool {
        true
    }

    #[expect(
        clippy::panic,
        reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
    )]
    fn action(document: &str, target: &str) -> LinkAction {
        resolve_link(Path::new(document), target, &FakeFiles::default(), yes)
            .unwrap_or_else(|error| panic!("{document} {target}: {error:?}"))
    }

    #[expect(
        clippy::panic,
        reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
    )]
    fn error(document: &str, target: &str) -> LinkError {
        match resolve_link(Path::new(document), target, &FakeFiles::default(), yes) {
            Ok(action) => panic!("{document} {target}: {action:?}"),
            Err(error) => error,
        }
    }

    fn show(path: &str, heading: Option<&str>) -> LinkAction {
        LinkAction::ShowDocument {
            path: PathBuf::from(path),
            heading: heading.map(str::to_owned),
        }
    }

    #[test]
    fn markdown_link_shows_the_document_in_this_window() {
        for (target, path) in [
            ("guide.md", r"C:\docs\guide\guide.md"),
            ("./sub/a.MD", r"C:\docs\guide\sub\a.MD"),
            (r"..\b.mdown", r"C:\docs\b.mdown"),
            ("my%20doc.md", r"C:\docs\guide\my doc.md"),
            ("my doc.md", r"C:\docs\guide\my doc.md"),
            ("C:/x.md", r"C:\x.md"),
            (r"C:\x.markdown", r"C:\x.markdown"),
            ("file:///C:/x.md", r"C:\x.md"),
            ("/top.md", r"C:\top.md"),
            ("guide.md?plain=1", r"C:\docs\guide\guide.md"),
        ] {
            assert_eq!(action(DOCUMENT, target), show(path, None), "{target}");
        }
    }

    #[test]
    fn markdown_link_with_a_heading_shows_the_document_at_that_heading() {
        for (target, path, heading) in [
            ("guide.md#install", r"C:\docs\guide\guide.md", "install"),
            ("a.md#gr%C3%B6%C3%9Fe", r"C:\docs\guide\a.md", "größe"),
            ("a.md#größe", r"C:\docs\guide\a.md", "größe"),
            ("C:/x.md#install", r"C:\x.md", "install"),
            ("file:///C:/x.md#install", r"C:\x.md", "install"),
        ] {
            assert_eq!(
                action(DOCUMENT, target),
                show(path, Some(heading)),
                "{target}"
            );
        }
    }

    #[test]
    fn markdown_link_with_an_empty_heading_shows_the_document_at_the_top() {
        assert_eq!(
            action(DOCUMENT, "guide.md#"),
            show(r"C:\docs\guide\guide.md", None)
        );
    }

    #[test]
    fn heading_link_scrolls_to_the_heading() {
        for (target, heading) in [
            ("#install", "install"),
            ("#fn-1", "fn-1"),
            ("#fnref-1", "fnref-1"),
            ("#%C3%BCber-uns", "über-uns"),
            ("#über-uns", "über-uns"),
        ] {
            assert_eq!(
                action(DOCUMENT, target),
                LinkAction::ScrollToHeading {
                    heading: Some(heading.to_owned())
                },
                "{target}"
            );
        }
    }

    #[test]
    fn empty_heading_link_scrolls_to_the_top() {
        for target in ["#", " # "] {
            assert_eq!(
                action(DOCUMENT, target),
                LinkAction::ScrollToHeading { heading: None },
                "{target:?}"
            );
        }
    }

    #[test]
    fn web_and_mail_links_go_to_the_system() {
        for (target, url) in [
            (
                "https://example.com/docs?q=1#top",
                "https://example.com/docs?q=1#top",
            ),
            ("http://example.com", "http://example.com/"),
            ("HTTPS://Example.com/a", "https://example.com/a"),
            (
                "https://example.com/a b\"c",
                "https://example.com/a%20b%22c",
            ),
            ("mailto:lamp@example.com", "mailto:lamp@example.com"),
            (
                "MAILTO:lamp@example.com?subject=Lamp",
                "mailto:lamp@example.com?subject=Lamp",
            ),
            (
                "mailto:lamp@example.com\" -x \"y",
                "mailto:lamp@example.com%22%20-x%20%22y",
            ),
        ] {
            assert_eq!(
                action(DOCUMENT, target),
                LinkAction::OpenUrl(url.to_owned()),
                "{target}"
            );
        }
    }

    #[test]
    fn local_file_that_is_not_markdown_is_revealed() {
        for (target, path) in [
            ("notes.txt", r"C:\docs\guide\notes.txt"),
            ("tool.exe", r"C:\docs\guide\tool.exe"),
            (r"..\run.bat", r"C:\docs\run.bat"),
            ("C:/apps/tool.lnk", r"C:\apps\tool.lnk"),
            ("LICENSE", r"C:\docs\guide\LICENSE"),
            ("notes.md.exe", r"C:\docs\guide\notes.md.exe"),
            ("file:///C:/docs/notes.txt", r"C:\docs\notes.txt"),
        ] {
            assert_eq!(
                action(DOCUMENT, target),
                LinkAction::RevealFile(PathBuf::from(path)),
                "{target}"
            );
        }
    }

    #[test]
    fn link_to_a_folder_opens_it() {
        for (target, path) in [
            ("docs/", r"C:\docs\guide\docs\"),
            ("..", r"C:\docs\"),
            ("../", r"C:\docs\"),
            (r"C:\", r"C:\"),
            ("file:///C:/docs/", r"C:\docs\"),
            ("folder.md", r"C:\docs\guide\folder.md"),
        ] {
            assert_eq!(
                action(DOCUMENT, target),
                LinkAction::OpenFolder(PathBuf::from(path)),
                "{target}"
            );
        }
    }

    #[test]
    fn links_with_other_schemes_do_nothing() {
        for target in [
            "ftp://example.com/a.md",
            "ms-settings:privacy",
            "search-ms:query=x",
            "javascript:alert(1)",
            " javascript:alert(1)",
            "data:text/html,x",
            "vbscript:x",
            "shell:startup",
            "http:",
            "",
            "   ",
            "?q",
        ] {
            assert_eq!(action(DOCUMENT, target), LinkAction::Nothing, "{target:?}");
        }
    }

    #[test]
    fn network_share_paths_resolve_like_local_paths_once_the_reader_says_yes() {
        for (document, target, expected) in [
            (
                DOCUMENT,
                r"\\server\share\a.md",
                show(r"\\server\share\a.md", None),
            ),
            (
                DOCUMENT,
                "%5C%5Cserver%5Cshare%5Ca.md",
                show(r"\\server\share\a.md", None),
            ),
            (
                DOCUMENT,
                "//server/share/b.txt",
                LinkAction::RevealFile(PathBuf::from(r"\\server\share\b.txt")),
            ),
            (
                DOCUMENT,
                "file://server/share/docs/",
                LinkAction::OpenFolder(PathBuf::from(r"\\server\share\docs\")),
            ),
            (
                r"\\server\share\docs\README.md",
                "guide.md#install",
                show(r"\\server\share\docs\guide.md", Some("install")),
            ),
            (
                r"\\server\share\docs\README.md",
                "../other/notes.txt",
                LinkAction::RevealFile(PathBuf::from(r"\\server\share\other\notes.txt")),
            ),
        ] {
            assert_eq!(action(document, target), expected, "{document} {target}");
        }
    }

    #[test]
    fn link_to_a_missing_file_is_an_error_naming_it() {
        for (target, path) in [
            ("missing.md", r"C:\docs\guide\missing.md"),
            ("missing.md#install", r"C:\docs\guide\missing.md"),
            ("../missing.txt", r"C:\docs\missing.txt"),
        ] {
            let error = error(DOCUMENT, target);
            assert!(
                matches!(&error, LinkError::NotFound { path: missing } if missing == Path::new(path)),
                "{target}: {error:?}"
            );
        }
    }

    #[test]
    fn link_to_a_path_that_cannot_be_reached_is_an_error_naming_it() {
        // An offline share or a file the reader may not open could be there, so the error must
        // not say it is missing.
        for (document, target, path) in [
            (DOCUMENT, r"\\offline\share\a.md", r"\\offline\share\a.md"),
            (
                DOCUMENT,
                "file://offline/share/docs/",
                r"\\offline\share\docs\",
            ),
            (
                r"\\offline\share\docs\README.md",
                "../notes.txt",
                r"\\offline\share\notes.txt",
            ),
        ] {
            let error = error(document, target);
            assert!(
                matches!(
                    &error,
                    LinkError::Unreachable { path: unreachable, source }
                        if unreachable == Path::new(path)
                            && source.kind() == io::ErrorKind::PermissionDenied
                ),
                "{document} {target}: {error:?}"
            );
        }
    }

    #[test]
    fn heading_and_web_links_look_up_no_file() {
        for target in [
            "#install",
            "https://example.com/",
            "mailto:lamp@example.com",
            "ftp://example.com/a.md",
            "",
            "?q",
        ] {
            let files = FakeFiles::default();
            let _ = resolve_link(Path::new(DOCUMENT), target, &files, |path, _| {
                panic!("asked to connect for {}", path.display())
            });
            assert!(files.asked.borrow().is_empty(), "{target:?}");
        }
    }

    /// Follows `target` in `document` with `answer` as the reader's answer, and gives what it did,
    /// what the reader was asked (the path and the host), and whether anything looked at a share.
    #[expect(
        clippy::panic,
        reason = "a helper isn't a #[test] function, and its panic fails the test that called it"
    )]
    fn follow_answering(
        document: &str,
        target: &str,
        files: &FakeFiles,
        answer: bool,
    ) -> (LinkAction, Vec<(PathBuf, String)>, bool) {
        let mut asked = Vec::new();
        let action = resolve_link(Path::new(document), target, files, |path, host| {
            asked.push((path.to_owned(), host.to_owned()));
            answer
        })
        .unwrap_or_else(|error| panic!("{document} {target}: {error:?}"));
        (action, asked, files.touched_a_share())
    }

    #[test]
    fn link_to_a_share_asks_naming_its_host_and_a_no_does_nothing_without_looking_there() {
        for (document, target, path) in [
            (DOCUMENT, r"\\server\share\a.md", r"\\server\share\a.md"),
            (DOCUMENT, "//server/share/b.txt", r"\\server\share\b.txt"),
            (
                DOCUMENT,
                "file://server/share/docs/",
                r"\\server\share\docs\",
            ),
            (
                r"\\other\share\docs\README.md",
                r"\\server\share\a.md",
                r"\\server\share\a.md",
            ),
            (
                r"\\?\UNC\other\share\docs\README.md",
                "file://server/share/docs/",
                r"\\server\share\docs\",
            ),
            (
                r"\\server.example\share\docs\README.md",
                r"\\server\share\a.md",
                r"\\server\share\a.md",
            ),
        ] {
            let files = FakeFiles::default();
            let (action, asked, touched) = follow_answering(document, target, &files, false);
            assert_eq!(action, LinkAction::Nothing, "{document} {target}");
            assert_eq!(
                asked,
                [(PathBuf::from(path), "server".to_owned())],
                "{document} {target}"
            );
            assert!(!touched, "{document} {target}: {:?}", files.asked);
        }
    }

    #[test]
    fn link_to_a_share_is_followed_after_a_yes() {
        let files = FakeFiles::default();
        let (action, asked, _) = follow_answering(DOCUMENT, r"\\server\share\a.md", &files, true);
        assert_eq!(action, show(r"\\server\share\a.md", None));
        assert_eq!(asked.len(), 1);
    }

    #[test]
    fn link_through_a_local_link_to_a_share_asks_naming_the_share_s_host() {
        let files = FakeFiles::default().with_link(r"C:\docs\guide\shared", r"\\nas\team");
        let (action, asked, touched) = follow_answering(DOCUMENT, "shared/a.md", &files, false);
        assert_eq!(action, LinkAction::Nothing);
        assert_eq!(
            asked,
            [(
                PathBuf::from(r"C:\docs\guide\shared\a.md"),
                "nas".to_owned()
            )]
        );
        assert!(!touched, "{:?}", files.asked);
    }

    #[test]
    fn link_to_a_share_on_the_document_s_own_host_is_followed_without_asking() {
        // Windows connected to the document's host to read the document, so following a link
        // there sends it nothing it hasn't had (ticket 20).
        for (document, target, expected) in [
            (
                r"\\server\share\docs\README.md",
                "guide.md#install",
                show(r"\\server\share\docs\guide.md", Some("install")),
            ),
            (
                r"\\server\share\docs\README.md",
                r"\\server\other\a.md",
                show(r"\\server\other\a.md", None),
            ),
            (
                r"\\SERVER\share\docs\README.md",
                "//Server/share/b.txt",
                LinkAction::RevealFile(PathBuf::from(r"\\server\share\b.txt")),
            ),
            (
                r"\\?\UNC\server\share\docs\README.md",
                r"\\Server\share\a.md",
                show(r"\\server\share\a.md", None),
            ),
            (
                r"\\Server\share\docs\README.md",
                "file://SERVER/share/docs/",
                LinkAction::OpenFolder(PathBuf::from(r"\\server\share\docs\")),
            ),
        ] {
            let files = FakeFiles::default();
            let (action, asked, _) = follow_answering(document, target, &files, false);
            assert_eq!(action, expected, "{document} {target}");
            assert_eq!(asked, [], "{document} {target}");
        }
    }

    #[test]
    fn link_through_a_local_link_to_the_document_s_own_host_is_followed_without_asking() {
        let files = FakeFiles::default().with_link(r"C:\shared", r"\\server\team");
        let (action, asked, _) = follow_answering(
            r"\\server\share\docs\README.md",
            "C:/shared/a.md",
            &files,
            false,
        );
        assert_eq!(action, show(r"C:\shared\a.md", None));
        assert_eq!(asked, []);
    }

    #[test]
    fn link_on_this_computer_asks_nothing() {
        let files = FakeFiles::default().with_link(r"C:\docs\guide\pics", r"D:\pics");
        for target in ["guide.md", "pics/a.png", "C:/x.md"] {
            let (_, asked, _) = follow_answering(DOCUMENT, target, &files, false);
            assert_eq!(asked, [], "{target}");
        }
    }

    #[test]
    fn link_through_a_local_link_to_a_device_does_nothing_without_asking() {
        let files = FakeFiles::default().with_link(r"C:\docs\guide\pipe", r"\\.\pipe\x");
        let (action, asked, _) = follow_answering(DOCUMENT, "pipe/a.md", &files, true);
        assert_eq!(action, LinkAction::Nothing);
        assert_eq!(asked, []);
    }
}
