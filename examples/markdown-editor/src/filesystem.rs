//! Filesystem boundary for the Markdown editor.
//!
//! The service validates the workspace root and discovers safe directory
//! entries below it. Every navigation target is canonicalized before use so
//! traversal and symlink resolution cannot escape the configured boundary.

use std::{
    cmp::Ordering,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use crate::domain::{DirectoryListing, ExplorerEntry, ExplorerEntryKind, Workspace};

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
        let canonical_root =
            canonicalize_with_context(requested_root, "failed to resolve browsing root")?;
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

    /// Discovers safe explorer entries in a directory below a workspace root.
    ///
    /// The requested directory and every retained entry target are
    /// canonicalized. Broken symlinks and symlinks that resolve outside the
    /// workspace are omitted so they cannot block access to valid entries.
    ///
    /// # Arguments
    ///
    /// * `workspace` — Validated root that bounds discovery.
    /// * `requested_directory` — Directory to canonicalize and list.
    ///
    /// # Returns
    ///
    /// A [`DirectoryListing`] containing directories followed by Markdown
    /// files in deterministic name order.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the directory cannot be resolved, lies outside
    /// the workspace, is not a directory, or cannot be read.
    pub(crate) fn list_directory(
        &self,
        workspace: &Workspace,
        requested_directory: &Path,
    ) -> io::Result<DirectoryListing> {
        let canonical_directory =
            canonicalize_with_context(requested_directory, "failed to resolve directory")?;
        ensure_within_root(workspace.root(), &canonical_directory)?;

        let metadata = fs::metadata(&canonical_directory).map_err(|source| {
            path_error(source, "failed to inspect directory", &canonical_directory)
        })?;
        if !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "explorer path is not a directory: {}",
                    canonical_directory.display()
                ),
            ));
        }

        let read_directory = fs::read_dir(&canonical_directory).map_err(|source| {
            path_error(source, "failed to read directory", &canonical_directory)
        })?;
        let mut entries = Vec::new();

        for entry_result in read_directory {
            let entry = entry_result.map_err(|source| {
                path_error(
                    source,
                    "failed to read directory entry",
                    &canonical_directory,
                )
            })?;
            let visible_path = entry.path();
            let file_type = entry.file_type().map_err(|source| {
                path_error(source, "failed to inspect directory entry", &visible_path)
            })?;
            let canonical_target = match fs::canonicalize(&visible_path) {
                Ok(path) => path,
                Err(_) if file_type.is_symlink() => continue,
                Err(source) => {
                    return Err(path_error(
                        source,
                        "failed to resolve directory entry",
                        &visible_path,
                    ));
                }
            };

            if !canonical_target.starts_with(workspace.root()) {
                continue;
            }

            let target_metadata = fs::metadata(&canonical_target).map_err(|source| {
                path_error(
                    source,
                    "failed to inspect directory entry target",
                    &visible_path,
                )
            })?;
            let name = entry.file_name();
            let kind = if target_metadata.is_dir() {
                ExplorerEntryKind::Directory
            } else if target_metadata.is_file() && is_markdown_name(&name) {
                ExplorerEntryKind::Markdown
            } else {
                continue;
            };

            entries.push(ExplorerEntry::new(name, canonical_target, kind));
        }

        entries.sort_by(compare_entries);
        Ok(DirectoryListing::new(canonical_directory, entries))
    }
}

/// Canonicalizes a path while retaining it in the error message.
///
/// # Arguments
///
/// * `path` — Path that should resolve to an existing filesystem entry.
/// * `operation` — Description of the canonicalization operation.
///
/// # Returns
///
/// A canonical absolute [`PathBuf`].
///
/// # Errors
///
/// Returns [`io::Error`] if the path cannot be canonicalized.
fn canonicalize_with_context(path: &Path, operation: &str) -> io::Result<PathBuf> {
    fs::canonicalize(path).map_err(|source| path_error(source, operation, path))
}

/// Rejects a canonical path that lies outside a workspace root.
///
/// # Arguments
///
/// * `root` — Canonical workspace boundary.
/// * `path` — Canonical path requested by the explorer.
///
/// # Returns
///
/// An empty [`Result`] when `path` is contained by `root`.
///
/// # Errors
///
/// Returns [`io::ErrorKind::PermissionDenied`] if `path` lies outside `root`.
fn ensure_within_root(root: &Path, path: &Path) -> io::Result<()> {
    if path.starts_with(root) {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "explorer path is outside browsing root '{}': {}",
            root.display(),
            path.display()
        ),
    ))
}

/// Returns whether a directory-entry name has a Markdown extension.
///
/// # Arguments
///
/// * `name` — Original filesystem entry name.
///
/// # Returns
///
/// A boolean indicating whether the extension is `md` or `markdown`, ignoring
/// ASCII case.
fn is_markdown_name(name: &OsStr) -> bool {
    Path::new(name)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

/// Compares explorer entries in deterministic display order.
///
/// Directories sort before Markdown files. Names then sort by their lossy
/// lowercase display value, with the original operating-system string as a
/// stable tie-breaker.
///
/// # Arguments
///
/// * `left` — Explorer entry on the left side of the comparison.
/// * `right` — Explorer entry on the right side of the comparison.
///
/// # Returns
///
/// An [`Ordering`] suitable for sorting explorer entries.
fn compare_entries(left: &ExplorerEntry, right: &ExplorerEntry) -> Ordering {
    entry_kind_rank(left.kind())
        .cmp(&entry_kind_rank(right.kind()))
        .then_with(|| {
            left.name()
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.name().to_string_lossy().to_lowercase())
        })
        .then_with(|| left.name().cmp(right.name()))
}

/// Returns the directory-first sort rank for an explorer entry kind.
///
/// # Arguments
///
/// * `kind` — Explorer entry classification to rank.
///
/// # Returns
///
/// A numeric rank with directories before Markdown files.
const fn entry_kind_rank(kind: ExplorerEntryKind) -> u8 {
    match kind {
        ExplorerEntryKind::Directory => 0,
        ExplorerEntryKind::Markdown => 1,
    }
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
