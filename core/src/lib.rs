//! Downlight's core: decoding and rendering documents, and resolving their links. It has no Tauri
//! dependency and knows nothing about windows, so the app calls it and its tests run without a
//! webview.

#![forbid(unsafe_code)]

mod decode;
mod file_url;
mod format;
mod highlight;
mod image;
mod links;
mod render;
mod sanitize;

pub use format::{MARKDOWN_EXTENSIONS, is_markdown};
pub use image::{LocalImage, local_image};
pub use links::{LinkAction, LinkError, PathKind, resolve_link};
pub use render::{RenderedDocument, render};
