//! Downlight's core: decoding and rendering documents, resolving their links and copying their
//! Markdown. It has no Tauri dependency and knows nothing about windows, so the app calls it and
//! its tests run without a webview.

#![forbid(unsafe_code)]

mod copy;
mod decode;
mod file_url;
mod files;
mod format;
mod highlight;
mod image;
mod links;
mod render;
mod sanitize;

pub use copy::{CopyError, Piece, markdown_of};
pub use files::{FileSystem, PathKind};
pub use format::{MARKDOWN_EXTENSIONS, TEXT_CHECK_LENGTH, is_markdown, is_text};
pub use image::{LocalImage, local_image};
pub use links::{LinkAction, LinkError, resolve_link};
pub use render::{Leave, RenderError, RenderedDocument, render};
