//! Downlight's core: decoding and rendering documents. It has no Tauri dependency and knows
//! nothing about windows, so the app calls it and its tests run without a webview.

#![forbid(unsafe_code)]

mod render;

pub use render::{RenderedDocument, render};
