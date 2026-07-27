//! Domain values owned by the Markdown editor.
//!
//! Domain types carry validated application state without performing
//! filesystem, process, or terminal operations.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

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

/// Kind of filesystem entry displayed by the Markdown explorer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExplorerEntryKind {
    /// Directory that can become the explorer's current location.
    Directory,
    /// Markdown document that can be opened by the preview.
    Markdown,
}

/// Safe filesystem entry discovered below a validated workspace root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExplorerEntry {
    /// Name shown in the explorer for the directory entry.
    name: OsString,
    /// Canonical absolute path to the entry target.
    path: PathBuf,
    /// Application-level classification of the entry.
    kind: ExplorerEntryKind,
}

impl ExplorerEntry {
    /// Creates a discovered explorer entry.
    ///
    /// # Arguments
    ///
    /// * `name` — Filesystem name shown to the user.
    /// * `path` — Canonical absolute target path.
    /// * `kind` — Directory or Markdown classification.
    ///
    /// # Returns
    ///
    /// An [`ExplorerEntry`] containing the safe discovered target.
    pub(crate) fn new(name: OsString, path: PathBuf, kind: ExplorerEntryKind) -> Self {
        Self { name, path, kind }
    }

    /// Returns the filesystem name shown in the explorer.
    ///
    /// # Returns
    ///
    /// An [`OsStr`] containing the original directory-entry name.
    pub(crate) fn name(&self) -> &OsStr {
        &self.name
    }

    /// Returns the explorer entry classification.
    ///
    /// # Returns
    ///
    /// An [`ExplorerEntryKind`] identifying a directory or Markdown file.
    pub(crate) const fn kind(&self) -> ExplorerEntryKind {
        self.kind
    }
}

/// Successful directory discovery below a workspace root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DirectoryListing {
    /// Canonical directory represented by this listing.
    directory: PathBuf,
    /// Safe display entries in deterministic explorer order.
    entries: Vec<ExplorerEntry>,
}

impl DirectoryListing {
    /// Creates a successful directory listing.
    ///
    /// # Arguments
    ///
    /// * `directory` — Canonical directory represented by the listing.
    /// * `entries` — Safe entries in display order.
    ///
    /// # Returns
    ///
    /// A [`DirectoryListing`] containing the discovered directory state.
    pub(crate) fn new(directory: PathBuf, entries: Vec<ExplorerEntry>) -> Self {
        Self { directory, entries }
    }

    /// Returns the canonical listed directory.
    ///
    /// # Returns
    ///
    /// A [`Path`] containing the current explorer directory.
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    /// Returns the ordered explorer entries.
    ///
    /// # Returns
    ///
    /// A slice of safe [`ExplorerEntry`] values.
    pub(crate) fn entries(&self) -> &[ExplorerEntry] {
        &self.entries
    }
}

/// Recoverable explorer state retained by the application controller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExplorerState {
    /// Last directory that loaded successfully.
    listing: DirectoryListing,
    /// Most recent navigation or discovery failure.
    error: Option<String>,
}

impl ExplorerState {
    /// Creates explorer state for a validated root before its first read.
    ///
    /// # Arguments
    ///
    /// * `root` — Canonical root used as the initial current directory.
    ///
    /// # Returns
    ///
    /// An empty [`ExplorerState`] anchored at `root`.
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            listing: DirectoryListing::new(root, Vec::new()),
            error: None,
        }
    }

    /// Returns the last successfully listed directory.
    ///
    /// # Returns
    ///
    /// A [`Path`] containing the current explorer directory.
    pub(crate) fn directory(&self) -> &Path {
        self.listing.directory()
    }

    /// Returns the entries from the last successful directory read.
    ///
    /// # Returns
    ///
    /// A slice of ordered [`ExplorerEntry`] values.
    pub(crate) fn entries(&self) -> &[ExplorerEntry] {
        self.listing.entries()
    }

    /// Returns the latest recoverable explorer error.
    ///
    /// # Returns
    ///
    /// An optional error message with operation and path context.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Replaces explorer contents after a successful directory read.
    ///
    /// # Arguments
    ///
    /// * `listing` — Newly discovered safe directory listing.
    pub(crate) fn replace_listing(&mut self, listing: DirectoryListing) {
        self.listing = listing;
        self.error = None;
    }

    /// Records a recoverable failure without discarding valid explorer data.
    ///
    /// # Arguments
    ///
    /// * `error` — Contextual error message to expose to the UI.
    pub(crate) fn record_error(&mut self, error: String) {
        self.error = Some(error);
    }
}
