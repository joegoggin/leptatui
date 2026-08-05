//! Markdown-specific policy layered over Leptatui's scoped filesystem.
//!
//! Leptatui owns path containment and asynchronous I/O. This module retains
//! only volume-boundary discovery, entry classification, filtering, and
//! directory-first presentation policy.

use std::{
    cmp::Ordering,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use leptatui::prelude::{FileEntry, FileKind};

/// Returns the filesystem or drive root containing an absolute path.
///
/// # Arguments
///
/// * `path` — Absolute path whose containment boundary should be discovered.
///
/// # Returns
///
/// A [`PathBuf`] containing the outermost ancestor of `path`.
pub(crate) fn volume_root(path: &Path) -> PathBuf {
    path.ancestors().last().unwrap_or(path).to_path_buf()
}

/// Kind of filesystem entry displayed by the Markdown explorer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExplorerEntryKind {
    /// Directory that can become the explorer's current location.
    Directory,
    /// Markdown document that can be opened by the preview.
    Markdown,
}

/// Safe filesystem entry displayed by the Markdown explorer.
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
    /// Returns the filesystem name shown to the user.
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

/// Successful directory discovery below a filesystem-volume root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DirectoryListing {
    /// Canonical directory represented by this listing.
    directory: PathBuf,
    /// Safe Markdown-editor entries in display order.
    entries: Vec<ExplorerEntry>,
}

impl DirectoryListing {
    /// Filters generic filesystem entries into Markdown explorer entries.
    ///
    /// # Arguments
    ///
    /// * `directory` — Canonical directory represented by the listing.
    /// * `entries` — Generic safe entries returned by Leptatui.
    ///
    /// # Returns
    ///
    /// A [`DirectoryListing`] containing directories followed by Markdown files.
    pub(crate) fn from_file_entries(directory: PathBuf, entries: Vec<FileEntry>) -> Self {
        let mut entries = entries
            .into_iter()
            .filter_map(|entry| {
                let kind = match entry.kind() {
                    FileKind::Directory => ExplorerEntryKind::Directory,
                    FileKind::File if is_markdown_name(entry.name()) => ExplorerEntryKind::Markdown,
                    FileKind::File | FileKind::Other => return None,
                };
                Some(ExplorerEntry {
                    name: entry.name().to_os_string(),
                    path: entry.path().to_path_buf(),
                    kind,
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by(compare_entries);
        Self { directory, entries }
    }

    /// Creates an empty listing for a validated directory.
    ///
    /// # Arguments
    ///
    /// * `directory` — Canonical directory represented by the empty listing.
    ///
    /// # Returns
    ///
    /// An empty [`DirectoryListing`] for `directory`.
    pub(crate) fn empty(directory: PathBuf) -> Self {
        Self {
            directory,
            entries: Vec::new(),
        }
    }

    /// Returns the listed directory.
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

/// Returns whether a path has a supported Markdown extension.
///
/// # Arguments
///
/// * `path` — Path whose final extension should be checked.
///
/// # Returns
///
/// A boolean indicating whether the extension is `md` or `markdown`.
pub(crate) fn is_markdown_path(path: &Path) -> bool {
    path.file_name().is_some_and(is_markdown_name)
}

/// Returns whether a directory-entry name has a Markdown extension.
///
/// # Arguments
///
/// * `name` — Original filesystem entry name.
///
/// # Returns
///
/// A boolean indicating whether the extension is `md` or `markdown`.
fn is_markdown_name(name: &OsStr) -> bool {
    Path::new(name)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

/// Compares explorer entries in deterministic directory-first display order.
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
    entry_kind_rank(left.kind)
        .cmp(&entry_kind_rank(right.kind))
        .then_with(|| {
            left.name
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.name.to_string_lossy().to_lowercase())
        })
        .then_with(|| left.name.cmp(&right.name))
}

/// Returns the directory-first rank for one explorer entry kind.
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
