//! What the core's integration tests share.

use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use downlight_core::{FileSystem, PathKind};

/// A file system with nothing in it, so that rendering doesn't depend on what is on the disk,
/// such as when an image's file was last saved.
pub struct Nothing;

impl FileSystem for Nothing {
    fn kind_of(&self, _: &Path) -> io::Result<Option<PathKind>> {
        Ok(None)
    }

    fn link_target(&self, _: &Path) -> io::Result<Option<PathBuf>> {
        Ok(None)
    }

    fn modified(&self, _: &Path) -> io::Result<Option<SystemTime>> {
        Ok(None)
    }
}
