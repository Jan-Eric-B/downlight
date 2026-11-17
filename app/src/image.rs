//! The image scheme, Downlight's own: the page loads a document's local images through it, never
//! from the file system directly. Rendering writes the images' URLs (the core's `ImageSources`);
//! the scheme serves the files they name.

use std::borrow::Cow;

use tauri::http::header::{
    CONTENT_SECURITY_POLICY, CONTENT_TYPE, HeaderValue, X_CONTENT_TYPE_OPTIONS,
};
use tauri::http::{Request, Response, StatusCode};
use tauri::{Runtime, UriSchemeContext, UriSchemeResponder};

use crate::disk::Disk;
use crate::platform;

/// The scheme's name, which the page sees as `http://image.localhost` on Windows.
pub const SCHEME: &str = "image";

/// Where the page loads local images from, the origin of the image scheme's URLs.
pub fn origin() -> String {
    platform::custom_scheme_origin(SCHEME)
}

/// An image is only ever shown in an `<img>`, where none of this applies. Should one be opened
/// on its own, an SVG would be a page: this keeps it from running scripts or loading anything.
const IMAGE_POLICY: &str = "default-src 'none'; style-src 'unsafe-inline'; sandbox";

/// Answers the page's request for a local image. The file is read on a blocking thread, off the
/// main thread, so a large image never holds up the windows.
pub fn serve<R: Runtime>(
    _: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let url_path = request.uri().path().to_owned();
    tauri::async_runtime::spawn_blocking(move || responder.respond(response_to(&url_path)));
}

/// The image a path names, or Not Found for anything that isn't a readable local image: a
/// missing image shows as missing in the page, as it does in a browser.
fn response_to(url_path: &str) -> Response<Cow<'static, [u8]>> {
    let image = downlight_core::local_image(url_path, &Disk);
    // The read follows links, after the check looked at them (ticket 20). A link swapped in
    // between gets past the check, but swapping it needs someone on this computer who can already
    // write next to the document, so the gap is accepted.
    let Some((image, bytes)) =
        image.and_then(|image| std::fs::read(image.path()).ok().map(|bytes| (image, bytes)))
    else {
        let mut response = Response::new(Cow::Borrowed(&[][..]));
        *response.status_mut() = StatusCode::NOT_FOUND;
        return response;
    };
    let mut response = Response::new(Cow::Owned(bytes));
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(image.media_type()));
    headers.insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(IMAGE_POLICY),
    );
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    response
}

// The policy names the origin WebView2 gives the scheme, which is Windows'.
#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn content_security_policy_lets_the_page_load_images_from_the_image_scheme() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let img_src = config["app"]["security"]["csp"]["img-src"]
            .as_str()
            .unwrap();
        assert!(
            img_src
                .split_ascii_whitespace()
                .any(|source| source == origin()),
            "{img_src}"
        );
    }
}
