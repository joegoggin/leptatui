//! Persistent recent-file storage for the Markdown editor.
//!
//! The store keeps a small versioned JSON document in the platform-local
//! application-data directory. Tests can replace that location or disable
//! persistence without changing reactive application behavior.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use super::is_markdown_path;

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
    /// In-memory history used when persistent storage is disabled.
    memory: Arc<Mutex<Vec<PathBuf>>>,
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

        Self {
            path,
            memory: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Creates a store that retains recent files only in application memory.
    ///
    /// # Returns
    ///
    /// A [`RecentFilesStore`] without an on-disk location.
    #[cfg(test)]
    pub(crate) fn memory() -> Self {
        Self {
            path: None,
            memory: Arc::new(Mutex::new(Vec::new())),
        }
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
        Self {
            path: Some(path),
            memory: Arc::new(Mutex::new(Vec::new())),
        }
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
            return Ok(self
                .memory
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone());
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

    /// Loads and validates globally visible recent Markdown paths.
    ///
    /// # Returns
    ///
    /// A tuple containing valid paths and an optional recoverable load error.
    pub(crate) fn load_valid(&self) -> (Vec<PathBuf>, Option<io::Error>) {
        let (stored_paths, error) = match self.load() {
            Ok(paths) => (paths, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        (valid_recent_paths(stored_paths), error)
    }

    /// Records one successfully opened Markdown file in bounded MRU order.
    ///
    /// Existing malformed history is replaced with a valid document containing
    /// the newly opened path.
    ///
    /// # Arguments
    ///
    /// * `path` — Successfully opened Markdown file.
    ///
    /// # Returns
    ///
    /// An empty [`io::Result`] after the updated history is persisted.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if `path` cannot be canonicalized, is not a
    /// Markdown file, or the updated history cannot be saved.
    pub(crate) fn record(&self, path: &Path) -> io::Result<()> {
        let canonical = fs::canonicalize(path)
            .map_err(|source| path_error(source, "failed to resolve recent file", path))?;
        if !fs::metadata(&canonical).is_ok_and(|metadata| metadata.is_file())
            || !is_markdown_path(&canonical)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("recent path is not a Markdown file: {}", path.display()),
            ));
        }

        let stored = self.load().unwrap_or_default();
        let mut entries = valid_recent_paths(stored);
        entries.retain(|entry| entry != &canonical);
        entries.insert(0, canonical);
        entries.truncate(RECENT_FILE_LIMIT);
        self.save(&entries)
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
            *self
                .memory
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = entries.to_vec();
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

/// Canonicalizes, filters, deduplicates, and bounds persisted recent paths.
///
/// # Arguments
///
/// * `paths` — Persisted paths in most-recent-first order.
///
/// # Returns
///
/// A [`Vec`] containing valid canonical Markdown paths.
fn valid_recent_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut valid = Vec::new();
    for path in paths {
        let Ok(canonical) = fs::canonicalize(path) else {
            continue;
        };
        let is_valid = fs::metadata(&canonical).is_ok_and(|metadata| metadata.is_file())
            && is_markdown_path(&canonical);
        if is_valid && !valid.contains(&canonical) {
            valid.push(canonical);
        }
        if valid.len() == RECENT_FILE_LIMIT {
            break;
        }
    }
    valid
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
