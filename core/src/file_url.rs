//! The files a document names: a relative or absolute path or a `file:` URL, as an image's source
//! or a link's target, resolved against the document's folder. Images (`image.rs`) and links
//! (`links.rs`) share it, so both read a path in a document the same way.

use std::path::Path;

use url::Url;

/// What a link target or an image's source in a document names, as [`classify`] reads it.
pub(crate) enum Target {
    /// A file or folder: a relative or absolute path, with either slash, or a `file:` URL, which
    /// [`resolve`] turns into its `file:` URL.
    File,
    /// A URL with any other scheme, which the caller allows or not.
    Url(Url),
    /// Neither, such as `http:` with no host, which names nothing.
    Unusable,
}

/// Reads `target` the way images and links both do, so a link and an image naming the same
/// thing name the same file.
pub(crate) fn classify(target: &str) -> Target {
    match Url::parse(target) {
        Ok(url) if url.scheme() == "file" || is_drive(url.scheme()) => Target::File,
        Ok(url) => Target::Url(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => Target::File,
        Err(_) => Target::Unusable,
    }
}

/// The folder of `document` as a `file:` URL, against which relative paths in it resolve. None
/// for a document with no folder, which launches, the open dialog and dropping never give.
pub(crate) fn folder_of(document: &Path) -> Option<Url> {
    document
        .parent()
        .and_then(|folder| Url::from_directory_path(folder).ok())
}

/// The `file:` URL that `target` names in a document in `folder`: a relative or absolute path,
/// with either slash, or a `file:` URL. The fragment stays. A path on a network share gives a URL
/// with a host; callers decide whether they want it.
pub(crate) fn resolve(folder: Option<&Url>, target: &str) -> Option<Url> {
    // comrak writes a backslash in a Markdown link as %5C, and Windows paths separate folders
    // with either slash.
    let target = target.replace("%5C", "/").replace("%5c", "/");
    if is_drive_path(&target) {
        Url::parse(&format!("file:///{target}"))
    } else {
        match folder {
            Some(folder) => folder.join(&target),
            None => Url::parse(&target),
        }
    }
    .ok()
}

/// Whether a URL's scheme is really a drive letter, as the URL parser reads `C:\pics\logo.png`.
fn is_drive(scheme: &str) -> bool {
    scheme.len() == 1 && scheme.bytes().all(|letter| letter.is_ascii_alphabetic())
}

/// Whether `target` starts with a drive letter and a slash, as `C:/pics` or `C:\pics` do.
fn is_drive_path(target: &str) -> bool {
    matches!(target.as_bytes(), [letter, b':', b'/' | b'\\', ..] if letter.is_ascii_alphabetic())
}
