//! Filesystem boundary for the Markdown editor.
//!
//! The initial service validates and canonicalizes the workspace root. Anchored
//! directory discovery and Markdown loading will extend this boundary without
//! placing operating-system access in the controller or UI.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::domain::Workspace;

/// Filesystem operations available to the application controller.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FileSystem;

impl FileSystem {
    /// Creates the filesystem service.
    ///
    /// # Returns
    ///
    /// A stateless [`FileSystem`] service.
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Validates and canonicalizes a requested workspace root.
    ///
    /// # Arguments
    ///
    /// * `requested_root` — User-selected path that should anchor browsing.
    ///
    /// # Returns
    ///
    /// A [`Workspace`] containing the canonical absolute root.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the path cannot be resolved, inspected, or is
    /// not a directory.
    pub(crate) fn validate_root(&self, requested_root: &Path) -> io::Result<Workspace> {
        let canonical_root = canonicalize_with_context(requested_root)?;
        let metadata = fs::metadata(&canonical_root).map_err(|source| {
            path_error(source, "failed to inspect browsing root", &canonical_root)
        })?;

        if !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "browsing root is not a directory: {}",
                    canonical_root.display()
                ),
            ));
        }

        Ok(Workspace::new(canonical_root))
    }
}

/// Canonicalizes a path while retaining it in the error message.
///
/// # Arguments
///
/// * `path` — Path that should resolve to an existing filesystem entry.
///
/// # Returns
///
/// A canonical absolute [`PathBuf`].
///
/// # Errors
///
/// Returns [`io::Error`] if the path cannot be canonicalized.
fn canonicalize_with_context(path: &Path) -> io::Result<PathBuf> {
    fs::canonicalize(path)
        .map_err(|source| path_error(source, "failed to resolve browsing root", path))
}

/// Adds operation and path context to a filesystem error.
///
/// # Arguments
///
/// * `source` — Original operating-system error.
/// * `operation` — Description of the failed filesystem operation.
/// * `path` — Path involved in the failure.
///
/// # Returns
///
/// An [`io::Error`] retaining the original error kind with a contextual
/// message.
fn path_error(source: io::Error, operation: &str, path: &Path) -> io::Error {
    io::Error::new(
        source.kind(),
        format!("{operation} '{}': {source}", path.display()),
    )
}
