//! Downlight's core: decoding and rendering documents. It has no Tauri dependency and knows
//! nothing about windows, so the app calls it and its tests run without a webview.

#![forbid(unsafe_code)]

mod decode;
mod format;
mod image;
mod render;
mod sanitize;

pub use format::{MARKDOWN_EXTENSIONS, is_markdown};
pub use image::{LocalImage, local_image};
pub use render::{RenderedDocument, render};
