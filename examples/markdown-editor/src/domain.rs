//! Domain values owned by the Markdown editor.
//!
//! Domain types carry validated application state without performing
//! filesystem, process, or terminal operations.

use std::path::{Path, PathBuf};

/// Validated directory that anchors one Markdown editing session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Workspace {
    /// Canonical absolute directory that bounds application browsing.
    root: PathBuf,
}

impl Workspace {
    /// Creates a workspace from a validated canonical root.
    ///
    /// # Arguments
    ///
    /// * `root` — Canonical absolute directory used as the browsing boundary.
    ///
    /// # Returns
    ///
    /// A [`Workspace`] anchored at the supplied directory.
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Returns the canonical browsing root.
    ///
    /// # Returns
    ///
    /// A [`Path`] containing the workspace boundary.
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}
