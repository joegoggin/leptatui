//! Domain values owned by the Markdown editor.
//!
//! Domain types carry validated application state without performing
//! filesystem, process, or terminal operations.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

/// Maximum number of recent files retained by the application.
pub(crate) const RECENT_FILE_LIMIT: usize = 10;

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

    /// Returns the canonical explorer target.
    ///
    /// # Returns
    ///
    /// A [`Path`] containing the directory or Markdown file target.
    pub(crate) fn path(&self) -> &Path {
        &self.path
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
    /// Currently highlighted explorer entry.
    selection: Option<usize>,
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
            selection: None,
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

    /// Returns the selected explorer index.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the selected entry index.
    pub(crate) const fn selection(&self) -> Option<usize> {
        self.selection
    }

    /// Returns the selected explorer entry.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the highlighted [`ExplorerEntry`].
    pub(crate) fn selected_entry(&self) -> Option<&ExplorerEntry> {
        self.selection
            .and_then(|selection| self.entries().get(selection))
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
        self.selection = (!listing.entries().is_empty()).then_some(0);
        self.listing = listing;
        self.error = None;
    }

    /// Moves the selection toward the beginning of the listing.
    pub(crate) fn select_previous(&mut self) {
        if let Some(selection) = &mut self.selection {
            *selection = selection.saturating_sub(1);
        }
    }

    /// Moves the selection toward the end of the listing.
    pub(crate) fn select_next(&mut self) {
        let last = self.entries().len().checked_sub(1);
        if let (Some(selection), Some(last)) = (&mut self.selection, last) {
            *selection = selection.saturating_add(1).min(last);
        }
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

/// Open Markdown document state owned by the editor application.
///
/// Markdown source loading and file-read failures belong to the path-backed
/// `Markdown` view. This state retains only the application concerns needed to
/// rebuild that view or retry an external editor operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PreviewState {
    /// Absolute Markdown path represented by the preview.
    path: Option<PathBuf>,
    /// Most recent external-editor failure.
    editor_error: Option<String>,
    /// Monotonic invalidation key for retained preview views.
    revision: u64,
}

impl PreviewState {
    /// Creates an empty preview.
    ///
    /// # Returns
    ///
    /// A [`PreviewState`] without an open Markdown file.
    pub(crate) const fn new() -> Self {
        Self {
            path: None,
            editor_error: None,
            revision: 0,
        }
    }

    /// Returns the open Markdown path.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the absolute preview path.
    pub(crate) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns the current external-editor error.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing a contextual editor failure.
    pub(crate) fn editor_error(&self) -> Option<&str> {
        self.editor_error.as_deref()
    }

    /// Returns the current preview invalidation revision.
    ///
    /// # Returns
    ///
    /// A [`u64`] that changes whenever retained Viewer state is invalidated.
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    /// Opens a Markdown path and invalidates the retained view.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute Markdown path represented by the document.
    pub(crate) fn open(&mut self, path: PathBuf) {
        self.path = Some(path);
        self.editor_error = None;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Records a recoverable external-editor failure.
    ///
    /// # Arguments
    ///
    /// * `error` — Contextual editor failure.
    pub(crate) fn record_editor_error(&mut self, error: String) {
        self.editor_error = Some(error);
        self.revision = self.revision.wrapping_add(1);
    }
}

/// Most-recently-used Markdown paths and recoverable persistence state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecentFilesState {
    /// Canonical paths in most-recent-first order.
    entries: Vec<PathBuf>,
    /// Most recent load or save failure.
    error: Option<String>,
}

impl RecentFilesState {
    /// Creates recent-file state from validated persisted paths.
    ///
    /// # Arguments
    ///
    /// * `entries` — Canonical paths in most-recent-first order.
    /// * `error` — Optional recoverable persistence error.
    ///
    /// # Returns
    ///
    /// A [`RecentFilesState`] capped at [`RECENT_FILE_LIMIT`] entries.
    pub(crate) fn new(mut entries: Vec<PathBuf>, error: Option<String>) -> Self {
        entries.truncate(RECENT_FILE_LIMIT);
        Self { entries, error }
    }

    /// Returns recent paths in most-recent-first order.
    ///
    /// # Returns
    ///
    /// A slice of canonical Markdown paths.
    pub(crate) fn entries(&self) -> &[PathBuf] {
        &self.entries
    }

    /// Returns the current recoverable persistence error.
    ///
    /// # Returns
    ///
    /// An optional error message.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Promotes a path to the front of the recent list.
    ///
    /// Existing copies are removed and the list remains bounded by
    /// [`RECENT_FILE_LIMIT`].
    ///
    /// # Arguments
    ///
    /// * `path` — Canonical Markdown path to promote.
    pub(crate) fn promote(&mut self, path: PathBuf) {
        self.entries.retain(|entry| entry != &path);
        self.entries.insert(0, path);
        self.entries.truncate(RECENT_FILE_LIMIT);
    }

    /// Removes a path from the recent list.
    ///
    /// # Arguments
    ///
    /// * `path` — Path that should no longer be offered.
    pub(crate) fn remove(&mut self, path: &Path) {
        self.entries.retain(|entry| entry != path);
    }

    /// Replaces the recoverable persistence error.
    ///
    /// # Arguments
    ///
    /// * `error` — New load or save error, or `None` after recovery.
    pub(crate) fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }
}
