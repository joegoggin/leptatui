//! Persistent recent-file storage for the Markdown editor.
//!
//! The store keeps a small versioned JSON document in the platform-local
//! application-data directory. Tests can replace that location or disable
//! persistence without changing reactive application behavior.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use super::{FileSystem, Workspace};

/// Current on-disk recent-file document version.
const DOCUMENT_VERSION: u8 = 1;
/// Maximum number of recent files retained by the application.
pub(crate) const RECENT_FILE_LIMIT: usize = 10;

/// Versioned recent-file document serialized to JSON.
#[derive(Debug, Deserialize, Serialize)]
struct RecentFilesDocument {
    /// Storage format version.
    version: u8,
    /// Canonical UTF-8 paths in most-recent-first order.
    entries: Vec<String>,
}

/// Persistence boundary for recent Markdown paths.
#[derive(Clone, Debug)]
pub(crate) struct RecentFilesStore {
    /// Optional JSON document path.
    path: Option<PathBuf>,
}

impl RecentFilesStore {
    /// Creates a store in the platform-local application-data directory.
    ///
    /// # Returns
    ///
    /// A [`RecentFilesStore`] using the standard application location, or a
    /// memory-only store when the platform cannot resolve one.
    pub(crate) fn standard() -> Self {
        let path = ProjectDirs::from("io.github", "joegoggin", "leptatui-markdown-editor")
            .map(|directories| directories.data_local_dir().join("recent-files.json"));

        Self { path }
    }

    /// Creates a store that retains recent files only in application memory.
    ///
    /// # Returns
    ///
    /// A [`RecentFilesStore`] without an on-disk location.
    #[cfg(test)]
    pub(crate) const fn memory() -> Self {
        Self { path: None }
    }

    /// Creates a store at an explicit JSON document path.
    ///
    /// # Arguments
    ///
    /// * `path` — File used to load and save recent paths.
    ///
    /// # Returns
    ///
    /// A [`RecentFilesStore`] using `path`.
    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    /// Loads persisted recent paths.
    ///
    /// A memory-only store and a missing document both produce an empty list.
    ///
    /// # Returns
    ///
    /// A [`Vec`] of stored paths in most-recent-first order.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the document cannot be read, parsed, or uses an
    /// unsupported version.
    pub(crate) fn load(&self) -> io::Result<Vec<PathBuf>> {
        let Some(path) = &self.path else {
            return Ok(Vec::new());
        };

        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(path_error(error, "failed to read recent files", path)),
        };
        let document: RecentFilesDocument = serde_json::from_str(&source).map_err(|source| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "failed to parse recent files '{}': {source}",
                    path.display()
                ),
            )
        })?;

        if document.version != DOCUMENT_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported recent-files version {} in '{}'",
                    document.version,
                    path.display()
                ),
            ));
        }

        Ok(document.entries.into_iter().map(PathBuf::from).collect())
    }

    /// Loads and filters recent paths for one workspace.
    ///
    /// # Arguments
    ///
    /// * `filesystem` — Service used to validate persisted Markdown paths.
    /// * `workspace` — Active workspace used to filter visible paths.
    ///
    /// # Returns
    ///
    /// A tuple containing workspace-visible paths, the complete persisted
    /// ordering, and an optional recoverable load error.
    pub(crate) fn load_for_workspace(
        &self,
        filesystem: FileSystem,
        workspace: &Workspace,
    ) -> (Vec<PathBuf>, Vec<PathBuf>, Option<io::Error>) {
        let (stored_paths, error) = match self.load() {
            Ok(paths) => (paths, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        let mut stored = Vec::new();
        for path in stored_paths {
            if !stored.contains(&path) {
                stored.push(path);
            }
        }
        stored.truncate(RECENT_FILE_LIMIT);

        let mut visible = Vec::new();
        for path in &stored {
            if let Ok(canonical) = filesystem.validate_markdown(workspace, path)
                && !visible.contains(&canonical)
            {
                visible.push(canonical);
            }
        }

        (visible, stored, error)
    }

    /// Persists recent paths in most-recent-first order.
    ///
    /// A memory-only store accepts the update without writing.
    ///
    /// # Arguments
    ///
    /// * `entries` — Canonical paths to serialize.
    ///
    /// # Returns
    ///
    /// An empty [`io::Result`] after a successful or memory-only save.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if a path is not UTF-8 or the storage directory,
    /// temporary document, serialization, or atomic replacement fails.
    pub(crate) fn save(&self, entries: &[PathBuf]) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };

        let serialized_entries = entries
            .iter()
            .map(|entry| {
                entry.to_str().map(str::to_owned).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "recent Markdown path is not valid UTF-8: {}",
                            entry.display()
                        ),
                    )
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        let document = RecentFilesDocument {
            version: DOCUMENT_VERSION,
            entries: serialized_entries,
        };
        let source = serde_json::to_string_pretty(&document).map_err(|source| {
            io::Error::other(format!("failed to serialize recent files: {source}"))
        })?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| {
                path_error(source, "failed to create recent-files directory", parent)
            })?;
        }

        let temporary = temporary_path(path);
        fs::write(&temporary, source).map_err(|source| {
            path_error(source, "failed to write temporary recent files", &temporary)
        })?;
        replace_file(&temporary, path)
    }
}

/// Returns a sibling temporary path for an atomic save.
///
/// # Arguments
///
/// * `path` — Final JSON document path.
///
/// # Returns
///
/// A [`PathBuf`] with a `.tmp` suffix.
fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

/// Replaces a persisted document with its prepared temporary file.
///
/// Windows does not replace an existing destination through [`fs::rename`],
/// so that platform removes the old application-state file first.
///
/// # Arguments
///
/// * `temporary` — Fully written sibling temporary file.
/// * `destination` — Final recent-files document path.
///
/// # Returns
///
/// An empty [`io::Result`] after the replacement succeeds.
///
/// # Errors
///
/// Returns [`io::Error`] if an existing Windows destination cannot be removed
/// or the temporary file cannot be renamed.
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(windows)]
    if destination.exists() {
        fs::remove_file(destination).map_err(|source| {
            path_error(
                source,
                "failed to remove previous recent files",
                destination,
            )
        })?;
    }

    fs::rename(temporary, destination)
        .map_err(|source| path_error(source, "failed to replace recent files", destination))
}

/// Adds operation and path context to a storage error.
///
/// # Arguments
///
/// * `source` — Original filesystem error.
/// * `operation` — Description of the failed operation.
/// * `path` — Path involved in the failure.
///
/// # Returns
///
/// An [`io::Error`] retaining the original error kind.
fn path_error(source: io::Error, operation: &str, path: &Path) -> io::Error {
    io::Error::new(
        source.kind(),
        format!("{operation} '{}': {source}", path.display()),
    )
}
