//! Root-scoped asynchronous filesystem access.
//!
//! The [`use_file_system`] hook validates a canonical directory boundary and
//! returns a [`FileSystem`] handle. Its methods immediately start typed
//! [`FileOperation`] values whose blocking work runs outside the terminal event thread. All
//! paths remain contained by the configured root, including after symlink
//! resolution. A leading `~` component expands to the current user's home
//! directory before roots and action paths are resolved.

use std::{
    ffi::{OsStr, OsString},
    fs::{self, OpenOptions},
    future::Future,
    io::{self, Write as _},
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use directories::BaseDirs;

use crate::{Action, executor::init_tokio_executor};

/// Counter used to create collision-resistant sibling temporary paths.
static TEMPORARY_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Observable handle for one already-started filesystem operation.
///
/// The action input is `()` because the filesystem method captures its path
/// and contents. Retain the handle to observe [`Action::pending`],
/// [`Action::value`], or [`Action::version`], or ignore it for fire-and-forget
/// work. Dispatching `()` retries the same captured operation.
pub type FileOperation<T> = Action<(), io::Result<T>>;

/// Configuration used when opening a root-scoped filesystem.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FileSystemOptions {
    /// Whether a missing root directory should be created.
    create_root: bool,
}

impl FileSystemOptions {
    /// Creates filesystem options with conservative defaults.
    ///
    /// # Returns
    ///
    /// A [`FileSystemOptions`] value that requires an existing root.
    pub const fn new() -> Self {
        Self { create_root: false }
    }

    /// Configures whether the scoped root may be created when missing.
    ///
    /// # Arguments
    ///
    /// * `create_root` — Whether initialization may create the root directory.
    ///
    /// # Returns
    ///
    /// A configured [`FileSystemOptions`] value.
    pub const fn create_root(mut self, create_root: bool) -> Self {
        self.create_root = create_root;
        self
    }
}

/// Generic filesystem entry classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileKind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// A filesystem entry that is neither a regular file nor a directory.
    Other,
}

/// Safe entry returned by a scoped directory listing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileEntry {
    /// Original name visible in the listed directory.
    name: OsString,
    /// Canonical target path contained by the filesystem root.
    path: PathBuf,
    /// Classification of the resolved target.
    kind: FileKind,
    /// Whether the visible directory entry is a symbolic link.
    symlink: bool,
}

impl FileEntry {
    /// Returns the original directory-entry name.
    ///
    /// # Returns
    ///
    /// An [`OsStr`] containing the visible entry name.
    pub fn name(&self) -> &OsStr {
        &self.name
    }

    /// Returns the canonical entry target.
    ///
    /// # Returns
    ///
    /// A [`Path`] contained by the scoped filesystem root.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the resolved entry classification.
    ///
    /// # Returns
    ///
    /// A [`FileKind`] describing the canonical target.
    pub const fn kind(&self) -> FileKind {
        self.kind
    }

    /// Returns whether the visible entry is a symbolic link.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the entry was discovered through a
    /// symbolic link.
    pub const fn is_symlink(&self) -> bool {
        self.symlink
    }
}

/// Portable metadata exposed by a scoped filesystem operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileMetadata {
    /// Classification of the resolved target.
    kind: FileKind,
    /// Length in bytes reported by the operating system.
    len: u64,
    /// Whether the target permissions are read-only.
    readonly: bool,
    /// Last modification time when the platform reports one.
    modified: Option<SystemTime>,
    /// Last access time when the platform reports one.
    accessed: Option<SystemTime>,
    /// Creation time when the platform reports one.
    created: Option<SystemTime>,
}

impl FileMetadata {
    /// Returns the resolved target classification.
    ///
    /// # Returns
    ///
    /// A [`FileKind`] describing the target.
    pub const fn kind(&self) -> FileKind {
        self.kind
    }

    /// Returns the target length in bytes.
    ///
    /// # Returns
    ///
    /// A byte count reported by the operating system.
    pub const fn len(&self) -> u64 {
        self.len
    }

    /// Returns whether the target is empty.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the reported length is zero.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns whether the target permissions are read-only.
    ///
    /// # Returns
    ///
    /// A boolean containing the portable read-only permission flag.
    pub const fn readonly(&self) -> bool {
        self.readonly
    }

    /// Returns the last modification time when available.
    ///
    /// # Returns
    ///
    /// An optional [`SystemTime`] reported by the platform.
    pub const fn modified(&self) -> Option<SystemTime> {
        self.modified
    }

    /// Returns the last access time when available.
    ///
    /// # Returns
    ///
    /// An optional [`SystemTime`] reported by the platform.
    pub const fn accessed(&self) -> Option<SystemTime> {
        self.accessed
    }

    /// Returns the creation time when available.
    ///
    /// # Returns
    ///
    /// An optional [`SystemTime`] reported by the platform.
    pub const fn created(&self) -> Option<SystemTime> {
        self.created
    }
}

/// Cloneable filesystem handle bounded by one canonical root directory.
///
/// Every operation expands a leading `~` or `~/` before enforcing root
/// containment. Named-user forms such as `~alice` remain literal paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSystem {
    /// Shared canonical root used by every operation.
    root: Arc<PathBuf>,
    /// Shared home directory used for leading-tilde expansion.
    home: Option<Arc<PathBuf>>,
}

impl FileSystem {
    /// Returns the canonical filesystem root.
    ///
    /// # Returns
    ///
    /// A [`Path`] containing the access boundary.
    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    /// Resolves an existing contained path to its canonical form.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute path to resolve.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] returning the canonical [`PathBuf`].
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn resolve_path(&self, path: impl Into<PathBuf>) -> FileOperation<PathBuf> {
        let filesystem = self.clone();
        let path = path.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            run_blocking(move || filesystem.resolve_existing(&path, "canonicalize path"))
        })
    }

    /// Reads portable metadata for an existing contained path.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute path to inspect.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] returning [`FileMetadata`].
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn get_metadata(&self, path: impl Into<PathBuf>) -> FileOperation<FileMetadata> {
        let filesystem = self.clone();
        let path = path.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            run_blocking(move || filesystem.get_metadata_blocking(&path))
        })
    }

    /// Lists safe contained directory entries.
    ///
    /// Broken symlinks and symlinks whose targets escape the root are omitted.
    /// Entries sort deterministically by case-insensitive display name with the
    /// original operating-system name as a tie-breaker.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute directory path to list.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] returning [`FileEntry`] values.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn read_dir(&self, path: impl Into<PathBuf>) -> FileOperation<Vec<FileEntry>> {
        let filesystem = self.clone();
        let path = path.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            run_blocking(move || filesystem.read_dir_blocking(&path))
        })
    }

    /// Reads a contained file as bytes.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute file path to read.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] returning the complete file contents.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn read_file_as_bytes(&self, path: impl Into<PathBuf>) -> FileOperation<Vec<u8>> {
        let filesystem = self.clone();
        let path = path.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            run_blocking(move || filesystem.read_file_as_bytes_blocking(&path))
        })
    }

    /// Reads a contained UTF-8 file as a string.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute file path to read and decode.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] returning a decoded [`String`].
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn read_file_as_string(&self, path: impl Into<PathBuf>) -> FileOperation<String> {
        let filesystem = self.clone();
        let path = path.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            run_blocking(move || filesystem.read_file_as_string_blocking(&path))
        })
    }

    /// Recursively creates a contained directory path.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute directory path to create.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] completing after creation.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn create_dir(&self, path: impl Into<PathBuf>) -> FileOperation<()> {
        let filesystem = self.clone();
        let path = path.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            run_blocking(move || filesystem.create_dir_blocking(&path))
        })
    }

    /// Creates or truncates a contained file and writes all supplied bytes.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute destination path.
    /// * `contents` — Bytes to write after truncating or creating the file.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] completing after the write.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn write_file(
        &self,
        path: impl Into<PathBuf>,
        contents: impl AsRef<[u8]>,
    ) -> FileOperation<()> {
        let filesystem = self.clone();
        let path = path.into();
        let contents = contents.as_ref().to_vec();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            let contents = contents.clone();
            run_blocking(move || {
                filesystem.write_file_blocking(&path, &contents, WriteMode::Truncate)
            })
        })
    }

    /// Creates a contained file or appends all supplied bytes to it.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute destination path.
    /// * `contents` — Bytes to append after opening or creating the file.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] completing after the append.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn append_file(
        &self,
        path: impl Into<PathBuf>,
        contents: impl AsRef<[u8]>,
    ) -> FileOperation<()> {
        let filesystem = self.clone();
        let path = path.into();
        let contents = contents.as_ref().to_vec();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            let contents = contents.clone();
            run_blocking(move || {
                filesystem.write_file_blocking(&path, &contents, WriteMode::Append)
            })
        })
    }

    /// Replaces a contained file through a sibling temporary file.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute destination path.
    /// * `contents` — Replacement bytes written before the destination changes.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] completing after replacement.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn write_and_replace_file(
        &self,
        path: impl Into<PathBuf>,
        contents: impl AsRef<[u8]>,
    ) -> FileOperation<()> {
        let filesystem = self.clone();
        let path = path.into();
        let contents = contents.as_ref().to_vec();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            let contents = contents.clone();
            run_blocking(move || filesystem.write_and_replace_file_blocking(&path, &contents))
        })
    }

    /// Copies a contained file to a new contained destination.
    ///
    /// # Arguments
    ///
    /// * `source` — Existing relative or contained absolute file path.
    /// * `destination` — New relative or contained absolute destination path.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] returning the copied byte count.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn copy_file(
        &self,
        source: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
    ) -> FileOperation<u64> {
        let filesystem = self.clone();
        let source = source.into();
        let destination = destination.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let source = source.clone();
            let destination = destination.clone();
            run_blocking(move || filesystem.copy_file_blocking(&source, &destination))
        })
    }

    /// Renames a contained file, directory, or symbolic link.
    ///
    /// The destination must not already exist. Both paths remain confined to
    /// the scoped filesystem root.
    ///
    /// # Arguments
    ///
    /// * `source` — Existing relative or contained absolute entry path.
    /// * `destination` — New relative or contained absolute destination path.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] completing after the rename.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn rename(
        &self,
        source: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
    ) -> FileOperation<()> {
        let filesystem = self.clone();
        let source = source.into();
        let destination = destination.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let source = source.clone();
            let destination = destination.clone();
            run_blocking(move || filesystem.rename_blocking(&source, &destination))
        })
    }

    /// Removes a contained file or symbolic link.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute file or symbolic-link path.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] completing after removal.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn delete_file(&self, path: impl Into<PathBuf>) -> FileOperation<()> {
        let filesystem = self.clone();
        let path = path.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            run_blocking(move || filesystem.delete_file_blocking(&path))
        })
    }

    /// Recursively removes a contained directory tree.
    ///
    /// The scoped root itself is always rejected. A symbolic-link input removes
    /// the link without traversing its target.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute directory path to remove.
    ///
    /// # Returns
    ///
    /// An already-started [`FileOperation`] completing after removal.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime.
    pub fn delete_dir(&self, path: impl Into<PathBuf>) -> FileOperation<()> {
        let filesystem = self.clone();
        let path = path.into();
        start_file_operation(move || {
            let filesystem = filesystem.clone();
            let path = path.clone();
            run_blocking(move || filesystem.delete_dir_blocking(&path))
        })
    }

    /// Resolves an existing path inside the canonical root.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute path to resolve.
    /// * `operation` — Description included in contextual errors.
    ///
    /// # Returns
    ///
    /// A canonical [`PathBuf`] contained by the root.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the path is invalid, missing, inaccessible, or
    /// escapes the root.
    fn resolve_existing(&self, path: &Path, operation: &str) -> io::Result<PathBuf> {
        reject_parent_components(path)?;
        let candidate = self.candidate(path)?;
        let canonical = fs::canonicalize(&candidate)
            .map_err(|source| path_error(source, operation, &candidate))?;
        self.ensure_contained(&canonical)?;
        Ok(canonical)
    }

    /// Resolves a contained destination through its nearest existing ancestor.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or contained absolute destination path.
    ///
    /// # Returns
    ///
    /// A resolved [`PathBuf`] that may not exist yet.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if an ancestor cannot be resolved or escapes the root.
    fn resolve_destination(&self, path: &Path) -> io::Result<PathBuf> {
        reject_parent_components(path)?;
        let candidate = self.candidate(path)?;
        if fs::symlink_metadata(&candidate).is_ok() {
            return self.resolve_existing(path, "resolve destination");
        }

        let mut ancestor = candidate.as_path();
        let mut suffix = Vec::new();
        while fs::symlink_metadata(ancestor).is_err() {
            let name = ancestor.file_name().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "destination escapes filesystem root: {}",
                        candidate.display()
                    ),
                )
            })?;
            suffix.push(name.to_os_string());
            ancestor = ancestor.parent().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "destination escapes filesystem root: {}",
                        candidate.display()
                    ),
                )
            })?;
        }

        let canonical_ancestor = fs::canonicalize(ancestor)
            .map_err(|source| path_error(source, "resolve destination ancestor", ancestor))?;
        self.ensure_contained(&canonical_ancestor)?;
        let mut resolved = canonical_ancestor;
        for name in suffix.into_iter().rev() {
            resolved.push(name);
        }
        Ok(resolved)
    }

    /// Resolves an entry without following its final symbolic link.
    ///
    /// # Arguments
    ///
    /// * `path` — Entry path requested by a destructive operation.
    ///
    /// # Returns
    ///
    /// A [`PathBuf`] whose parent is canonical and contained.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the entry has no safe contained parent.
    fn resolve_entry(&self, path: &Path) -> io::Result<PathBuf> {
        reject_parent_components(path)?;
        let candidate = self.candidate(path)?;
        if candidate == *self.root {
            return Ok(candidate);
        }
        let name = candidate.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "filesystem entry has no file name",
            )
        })?;
        let parent = candidate.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "filesystem entry has no parent",
            )
        })?;
        let canonical_parent = fs::canonicalize(parent)
            .map_err(|source| path_error(source, "resolve entry parent", parent))?;
        self.ensure_contained(&canonical_parent)?;
        Ok(canonical_parent.join(name))
    }

    /// Builds an absolute candidate path from a user input.
    ///
    /// A leading `~` component expands to the current user's home directory
    /// before relative paths are joined to the scoped root.
    ///
    /// # Arguments
    ///
    /// * `path` — Relative or absolute input path.
    ///
    /// # Returns
    ///
    /// An [`io::Result`] containing an absolute [`PathBuf`] before canonical
    /// resolution.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if the path requires tilde
    /// expansion and the current user's home directory is unavailable.
    fn candidate(&self, path: &Path) -> io::Result<PathBuf> {
        let path = expand_tilde(path, self.home.as_ref().map(|home| home.as_path()))?;
        if path.as_os_str().is_empty() {
            Ok(self.root.as_ref().clone())
        } else if path.is_absolute() {
            Ok(path)
        } else {
            Ok(self.root.join(path))
        }
    }

    /// Rejects a canonical path outside the filesystem root.
    ///
    /// # Arguments
    ///
    /// * `path` — Canonical path to check.
    ///
    /// # Returns
    ///
    /// An empty [`io::Result`] for a contained path.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::PermissionDenied`] if the path escapes the root.
    fn ensure_contained(&self, path: &Path) -> io::Result<()> {
        if path.starts_with(self.root.as_path()) {
            return Ok(());
        }
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "path is outside filesystem root '{}': {}",
                self.root.display(),
                path.display()
            ),
        ))
    }

    /// Reads portable metadata for one contained path.
    ///
    /// # Arguments
    ///
    /// * `path` — Existing path to inspect.
    ///
    /// # Returns
    ///
    /// A [`FileMetadata`] value derived from operating-system metadata.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if resolution or metadata inspection fails.
    fn get_metadata_blocking(&self, path: &Path) -> io::Result<FileMetadata> {
        let resolved = self.resolve_existing(path, "resolve metadata path")?;
        let metadata = fs::metadata(&resolved)
            .map_err(|source| path_error(source, "inspect path", &resolved))?;
        Ok(FileMetadata {
            kind: metadata_kind(&metadata),
            len: metadata.len(),
            readonly: metadata.permissions().readonly(),
            modified: metadata.modified().ok(),
            accessed: metadata.accessed().ok(),
            created: metadata.created().ok(),
        })
    }

    /// Lists safe entries below one contained directory.
    ///
    /// # Arguments
    ///
    /// * `path` — Existing directory path to list.
    ///
    /// # Returns
    ///
    /// A sorted vector of [`FileEntry`] values.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the directory cannot be resolved or read.
    fn read_dir_blocking(&self, path: &Path) -> io::Result<Vec<FileEntry>> {
        let directory = self.resolve_existing(path, "resolve directory")?;
        let metadata = fs::metadata(&directory)
            .map_err(|source| path_error(source, "inspect directory", &directory))?;
        if !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "filesystem path is not a directory: {}",
                    directory.display()
                ),
            ));
        }

        let mut entries = Vec::new();
        let read_dir = fs::read_dir(&directory)
            .map_err(|source| path_error(source, "read directory", &directory))?;
        for result in read_dir {
            let entry =
                result.map_err(|source| path_error(source, "read directory entry", &directory))?;
            let visible_path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|source| path_error(source, "inspect directory entry", &visible_path))?;
            let target = match fs::canonicalize(&visible_path) {
                Ok(target) if target.starts_with(self.root.as_path()) => target,
                Ok(_) => continue,
                Err(_) if file_type.is_symlink() => continue,
                Err(source) => {
                    return Err(path_error(source, "resolve directory entry", &visible_path));
                }
            };
            let metadata = fs::metadata(&target).map_err(|source| {
                path_error(source, "inspect directory entry target", &visible_path)
            })?;
            entries.push(FileEntry {
                name: entry.file_name(),
                path: target,
                kind: metadata_kind(&metadata),
                symlink: file_type.is_symlink(),
            });
        }
        entries.sort_by(|left, right| {
            left.name
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.name.to_string_lossy().to_lowercase())
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(entries)
    }

    /// Reads one contained file as bytes.
    ///
    /// # Arguments
    ///
    /// * `path` — Existing file path to read.
    ///
    /// # Returns
    ///
    /// A vector containing the complete file contents.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if resolution or reading fails.
    fn read_file_as_bytes_blocking(&self, path: &Path) -> io::Result<Vec<u8>> {
        let resolved = self.resolve_existing(path, "resolve file")?;
        fs::read(&resolved).map_err(|source| path_error(source, "read file", &resolved))
    }

    /// Reads one contained file as UTF-8 text.
    ///
    /// # Arguments
    ///
    /// * `path` — Existing file path to read.
    ///
    /// # Returns
    ///
    /// A [`String`] containing the decoded file contents.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if resolution, reading, or UTF-8 decoding fails.
    fn read_file_as_string_blocking(&self, path: &Path) -> io::Result<String> {
        let resolved = self.resolve_existing(path, "resolve text file")?;
        fs::read_to_string(&resolved)
            .map_err(|source| path_error(source, "read text file", &resolved))
    }

    /// Recursively creates one contained directory path.
    ///
    /// # Arguments
    ///
    /// * `path` — Destination directory path.
    ///
    /// # Returns
    ///
    /// An empty [`io::Result`] after creation succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if resolution or directory creation fails.
    fn create_dir_blocking(&self, path: &Path) -> io::Result<()> {
        let destination = self.resolve_destination(path)?;
        let result = fs::create_dir_all(&destination);
        result.map_err(|source| path_error(source, "create directory", &destination))
    }

    /// Writes a contained file with the requested ordinary write mode.
    ///
    /// # Arguments
    ///
    /// * `path` — Destination path.
    /// * `contents` — Bytes to write.
    /// * `mode` — Whether to truncate or append.
    ///
    /// # Returns
    ///
    /// An empty [`io::Result`] after writing succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if resolution, opening, or writing fails.
    fn write_file_blocking(&self, path: &Path, contents: &[u8], mode: WriteMode) -> io::Result<()> {
        let destination = self.resolve_destination(path)?;
        let mut options = OpenOptions::new();
        options.create(true).write(true);
        match mode {
            WriteMode::Truncate => {
                options.truncate(true);
            }
            WriteMode::Append => {
                options.append(true);
            }
        }
        let mut file = options
            .open(&destination)
            .map_err(|source| path_error(source, "open file for writing", &destination))?;
        file.write_all(contents)
            .map_err(|source| path_error(source, "write file", &destination))
    }

    /// Replaces a contained file through a prepared sibling temporary file.
    ///
    /// # Arguments
    ///
    /// * `path` — Destination path.
    /// * `contents` — Replacement bytes.
    ///
    /// # Returns
    ///
    /// An empty [`io::Result`] after replacement succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if resolution, temporary writing, or replacement fails.
    fn write_and_replace_file_blocking(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        let destination = self.resolve_destination(path)?;
        let parent = destination.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "write destination has no parent",
            )
        })?;
        let mut temporary = None;
        for _ in 0..32 {
            let sequence = TEMPORARY_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let name = format!(
                ".leptatui-{}-{}-{sequence}.tmp",
                std::process::id(),
                destination
                    .file_name()
                    .unwrap_or_else(|| OsStr::new("file"))
                    .to_string_lossy()
            );
            let candidate = parent.join(name);
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&candidate)
            {
                Ok(file) => {
                    temporary = Some((candidate, file));
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => {
                    return Err(path_error(source, "create temporary file", &candidate));
                }
            }
        }
        let (temporary_path, mut temporary_file) = temporary.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "failed to allocate temporary file beside '{}'",
                    destination.display()
                ),
            )
        })?;
        if let Err(source) = temporary_file.write_all(contents) {
            drop(temporary_file);
            let _ = fs::remove_file(&temporary_path);
            return Err(path_error(source, "write temporary file", &temporary_path));
        }
        drop(temporary_file);

        #[cfg(windows)]
        if fs::symlink_metadata(&destination).is_ok() {
            fs::remove_file(&destination).map_err(|source| {
                let _ = fs::remove_file(&temporary_path);
                path_error(source, "remove previous destination", &destination)
            })?;
        }

        fs::rename(&temporary_path, &destination).map_err(|source| {
            let _ = fs::remove_file(&temporary_path);
            path_error(source, "replace file", &destination)
        })
    }

    /// Copies one contained file to a new contained destination.
    ///
    /// # Arguments
    ///
    /// * `source` — Existing source file path.
    /// * `destination` — New destination path.
    ///
    /// # Returns
    ///
    /// The number of bytes copied.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if either path is invalid, the destination exists,
    /// or copying fails.
    fn copy_file_blocking(&self, source: &Path, destination: &Path) -> io::Result<u64> {
        let source = self.resolve_existing(source, "resolve copy source")?;
        let destination = self.resolve_new_destination(destination)?;
        fs::copy(&source, &destination)
            .map_err(|error| path_error(error, "copy file", &destination))
    }

    /// Renames one contained entry to a new contained destination.
    ///
    /// # Arguments
    ///
    /// * `source` — Existing source entry path.
    /// * `destination` — New destination path.
    ///
    /// # Returns
    ///
    /// An empty [`io::Result`] after the rename succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if either path is invalid, the destination exists,
    /// or renaming fails.
    fn rename_blocking(&self, source: &Path, destination: &Path) -> io::Result<()> {
        let source = self.resolve_entry(source)?;
        fs::symlink_metadata(&source)
            .map_err(|error| path_error(error, "inspect rename source", &source))?;
        if source == *self.root {
            return Err(root_mutation_error("rename"));
        }
        let destination = self.resolve_new_destination(destination)?;
        fs::rename(&source, &destination)
            .map_err(|error| path_error(error, "rename entry", &destination))
    }

    /// Resolves a destination and rejects any existing directory entry.
    ///
    /// # Arguments
    ///
    /// * `path` — Requested new destination path.
    ///
    /// # Returns
    ///
    /// A contained, currently absent [`PathBuf`].
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::AlreadyExists`] when the destination exists.
    fn resolve_new_destination(&self, path: &Path) -> io::Result<PathBuf> {
        let destination = self.resolve_destination(path)?;
        if fs::symlink_metadata(&destination).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "filesystem destination already exists: {}",
                    destination.display()
                ),
            ));
        }
        Ok(destination)
    }

    /// Removes one contained file or symbolic link.
    ///
    /// # Arguments
    ///
    /// * `path` — Existing entry path to remove.
    ///
    /// # Returns
    ///
    /// An empty [`io::Result`] after removal succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if resolution, inspection, or removal fails.
    fn delete_file_blocking(&self, path: &Path) -> io::Result<()> {
        let entry = self.resolve_entry(path)?;
        if entry == *self.root {
            return Err(root_mutation_error("remove"));
        }
        fs::remove_file(&entry).map_err(|source| path_error(source, "remove file", &entry))
    }

    /// Recursively removes one contained directory entry.
    ///
    /// Symbolic-link inputs are removed without traversing their targets.
    ///
    /// # Arguments
    ///
    /// * `path` — Existing directory or directory-symlink path to remove.
    /// # Returns
    ///
    /// An empty [`io::Result`] after removal succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the root is requested or removal fails.
    fn delete_dir_blocking(&self, path: &Path) -> io::Result<()> {
        let entry = self.resolve_entry(path)?;
        if entry == *self.root {
            return Err(root_mutation_error("remove"));
        }
        let metadata = fs::symlink_metadata(&entry)
            .map_err(|source| path_error(source, "inspect directory", &entry))?;
        let result = if metadata.file_type().is_symlink() {
            fs::remove_file(&entry)
        } else {
            fs::remove_dir_all(&entry)
        };
        result.map_err(|source| path_error(source, "remove directory", &entry))
    }
}

/// Ordinary file-writing behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WriteMode {
    /// Creates or truncates a destination.
    Truncate,
    /// Creates or appends to a destination.
    Append,
}

/// Creates a filesystem handle rooted at an existing directory.
///
/// A leading `~` or `~/` expands to the current user's home directory before
/// the root is canonicalized.
///
/// # Arguments
///
/// * `root` — Directory that bounds every filesystem operation, optionally
///   beginning with `~`.
///
/// # Returns
///
/// A canonical root-scoped [`FileSystem`].
///
/// # Errors
///
/// Returns [`io::Error`] if the root is missing, inaccessible, or not a directory.
pub fn use_file_system(root: impl AsRef<Path>) -> io::Result<FileSystem> {
    use_file_system_with_options(root, FileSystemOptions::default())
}

/// Creates a filesystem handle with explicit root-initialization options.
///
/// A leading `~` or `~/` expands to the current user's home directory before
/// optional creation and canonicalization.
///
/// # Arguments
///
/// * `root` — Directory that bounds every filesystem operation, optionally
///   beginning with `~`.
/// * `options` — Root creation behavior.
///
/// # Returns
///
/// A canonical root-scoped [`FileSystem`].
///
/// # Errors
///
/// Returns [`io::Error`] if creation, canonicalization, or validation fails.
pub fn use_file_system_with_options(
    root: impl AsRef<Path>,
    options: FileSystemOptions,
) -> io::Result<FileSystem> {
    init_tokio_executor();
    let home = BaseDirs::new().map(|directories| directories.home_dir().to_path_buf());
    initialize_file_system(root.as_ref(), options, home)
}

/// Initializes a filesystem with an explicitly discovered home directory.
///
/// # Arguments
///
/// * `root` — Requested root path before tilde expansion.
/// * `options` — Root creation behavior.
/// * `home` — Current user's home directory when discovery succeeded.
///
/// # Returns
///
/// A canonical root-scoped [`FileSystem`].
///
/// # Errors
///
/// Returns [`io::Error`] if expansion, creation, canonicalization, or
/// validation fails.
fn initialize_file_system(
    root: &Path,
    options: FileSystemOptions,
    home: Option<PathBuf>,
) -> io::Result<FileSystem> {
    let root = expand_tilde(root, home.as_deref())?;
    if options.create_root {
        fs::create_dir_all(&root)
            .map_err(|source| path_error(source, "create filesystem root", &root))?;
    }
    let canonical = fs::canonicalize(&root)
        .map_err(|source| path_error(source, "resolve filesystem root", &root))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|source| path_error(source, "inspect filesystem root", &canonical))?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "filesystem root is not a directory: {}",
                canonical.display()
            ),
        ));
    }
    Ok(FileSystem {
        root: Arc::new(canonical),
        home: home.map(Arc::new),
    })
}

/// Expands a leading home-directory component in one path.
///
/// Only exact `~` and `~/` forms expand. Named-user forms such as `~alice` and
/// tildes after the first component remain unchanged.
///
/// # Arguments
///
/// * `path` — Input path to inspect.
/// * `home` — Current user's home directory when discovery succeeded.
///
/// # Returns
///
/// An [`io::Result`] containing the expanded or unchanged [`PathBuf`].
///
/// # Errors
///
/// Returns [`io::ErrorKind::NotFound`] if expansion is required and `home` is
/// unavailable.
fn expand_tilde(path: &Path, home: Option<&Path>) -> io::Result<PathBuf> {
    let mut components = path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return Ok(path.to_path_buf());
    };
    if first != OsStr::new("~") {
        return Ok(path.to_path_buf());
    }
    let home = home.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "cannot expand filesystem path '{}' because the current user's home directory is unavailable",
                path.display()
            ),
        )
    })?;
    let mut expanded = home.to_path_buf();
    expanded.extend(components);
    Ok(expanded)
}

/// Creates and immediately dispatches an operation with captured arguments.
///
/// # Arguments
///
/// * `operation` — Repeatable asynchronous filesystem operation.
///
/// # Returns
///
/// An already-started [`FileOperation`].
///
/// # Panics
///
/// Panics if called outside a Tokio runtime.
fn start_file_operation<T, F, Fut>(operation: F) -> FileOperation<T>
where
    T: Send + Sync + 'static,
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = io::Result<T>> + Send + 'static,
{
    let action = Action::new(move |_: &()| operation());
    action.dispatch(());
    action
}

/// Runs a blocking filesystem operation through Tokio's blocking pool.
///
/// # Arguments
///
/// * `operation` — Synchronous filesystem operation to execute.
///
/// # Returns
///
/// A future resolving to the operation's [`io::Result`].
async fn run_blocking<T>(
    operation: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T>
where
    T: Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| io::Error::other(format!("filesystem task failed: {error}")))?
}

/// Rejects explicit parent-directory components before path resolution.
///
/// # Arguments
///
/// * `path` — User-supplied path to inspect.
///
/// # Returns
///
/// An empty [`io::Result`] when the path contains no parent traversal.
///
/// # Errors
///
/// Returns [`io::ErrorKind::PermissionDenied`] for any `..` component.
fn reject_parent_components(path: &Path) -> io::Result<()> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("parent traversal is not allowed: {}", path.display()),
        ));
    }
    Ok(())
}

/// Classifies portable operating-system metadata.
///
/// # Arguments
///
/// * `metadata` — Metadata to classify.
///
/// # Returns
///
/// A generic [`FileKind`] value.
fn metadata_kind(metadata: &fs::Metadata) -> FileKind {
    if metadata.is_file() {
        FileKind::File
    } else if metadata.is_dir() {
        FileKind::Directory
    } else {
        FileKind::Other
    }
}

/// Creates a root-mutation rejection error.
///
/// # Arguments
///
/// * `operation` — Destructive operation that was rejected.
///
/// # Returns
///
/// An [`io::Error`] describing the protected root.
fn root_mutation_error(operation: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("cannot {operation} the scoped filesystem root"),
    )
}

/// Adds operation and path context to an operating-system error.
///
/// # Arguments
///
/// * `source` — Original operating-system error.
/// * `operation` — Description of the failed operation.
/// * `path` — Path involved in the failure.
///
/// # Returns
///
/// An [`io::Error`] retaining the original error kind.
fn path_error(source: io::Error, operation: &str, path: &Path) -> io::Error {
    io::Error::new(
        source.kind(),
        format!("failed to {operation} '{}': {source}", path.display()),
    )
}

#[cfg(test)]
/// Tests for root containment and filesystem operation behavior.
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::Duration,
    };

    use leptos::prelude::{GetUntracked, Owner, WithUntracked};
    use tokio::{task::yield_now, time::timeout};

    use super::*;

    /// Counter used to isolate temporary filesystem tests.
    static TEST_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Temporary directory removed after one filesystem test.
    struct TestRoot {
        /// Unique temporary root path.
        path: PathBuf,
    }

    impl TestRoot {
        /// Creates a unique empty temporary directory.
        ///
        /// # Arguments
        ///
        /// * `label` — Human-readable suffix used by diagnostics.
        ///
        /// # Returns
        ///
        /// A [`TestRoot`] owning the created directory.
        fn new(label: &str) -> Self {
            let sequence = TEST_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "leptatui-file-system-{}-{sequence}-{label}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("temporary filesystem root should be created");
            Self { path }
        }

        /// Returns the temporary root path.
        ///
        /// # Returns
        ///
        /// A [`Path`] containing the temporary directory.
        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestRoot {
        /// Removes the temporary test directory and its contents.
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// Verifies filesystem initialization validates or creates canonical roots.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// existing/
    /// missing/ with create_root(true)
    /// regular-file
    /// ```
    ///
    /// # Assertions
    ///
    /// - An existing directory produces its canonical root.
    /// - Explicit root creation creates and opens a missing directory.
    /// - A regular file is rejected as a filesystem root.
    #[test]
    fn initialization_validates_and_optionally_creates_roots() {
        let tree = TestRoot::new("initialization");
        let filesystem = use_file_system(tree.path()).expect("existing root should open");
        assert_eq!(
            filesystem.root(),
            fs::canonicalize(tree.path()).expect("temporary filesystem root should canonicalize")
        );

        let created = tree.path().join("created");
        let created_filesystem =
            use_file_system_with_options(&created, FileSystemOptions::new().create_root(true))
                .expect("configured missing root should be created");
        assert_eq!(created_filesystem.root(), created);

        let regular = tree.path().join("regular");
        fs::write(&regular, "data").expect("regular root fixture should be written");
        let error = use_file_system(&regular).expect_err("regular file root should fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    /// Verifies tilde expansion recognizes only current-user path forms.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// ~
    /// ~/Projects
    /// ~alice
    /// nested/~/file
    /// ```
    ///
    /// # Assertions
    ///
    /// - Exact `~` expands to the supplied home directory.
    /// - A leading `~/` expands while preserving its suffix.
    /// - Named-user and embedded tilde components remain unchanged.
    /// - An absolute path remains unchanged.
    /// - Missing home discovery produces `NotFound` only when expansion is needed.
    #[test]
    fn tilde_expansion_recognizes_only_current_user_forms() {
        let home = Path::new("/virtual/home");
        assert_eq!(
            expand_tilde(Path::new("~"), Some(home)).expect("exact tilde should expand"),
            home
        );
        assert_eq!(
            expand_tilde(&Path::new("~").join("Projects"), Some(home))
                .expect("home-relative path should expand"),
            home.join("Projects")
        );
        assert_eq!(
            expand_tilde(Path::new("~alice"), Some(home))
                .expect("named-user form should remain literal"),
            Path::new("~alice")
        );
        assert_eq!(
            expand_tilde(Path::new("nested/~/file"), Some(home))
                .expect("embedded tilde should remain literal"),
            Path::new("nested/~/file")
        );
        assert_eq!(
            expand_tilde(Path::new("/already/absolute"), Some(home))
                .expect("absolute path should remain unchanged"),
            Path::new("/already/absolute")
        );
        assert_eq!(
            expand_tilde(Path::new("relative"), None)
                .expect("ordinary relative path should not need a home directory"),
            Path::new("relative")
        );
        let error = expand_tilde(Path::new("~"), None)
            .expect_err("tilde without a home directory should fail");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    /// Verifies tilde expansion preserves non-UTF-8 suffix components.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// ~/<non-UTF-8 component>
    /// ```
    ///
    /// # Assertions
    ///
    /// - The leading tilde expands to the supplied home directory.
    /// - The non-UTF-8 suffix bytes remain unchanged.
    #[cfg(unix)]
    #[test]
    fn tilde_expansion_preserves_non_utf8_suffixes() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        let home = Path::new("/virtual/home");
        let suffix = OsString::from_vec(vec![b'p', 0xff, b'h']);
        let input = Path::new("~").join(&suffix);
        let expanded = expand_tilde(&input, Some(home)).expect("tilde path should expand");

        assert_eq!(expanded, home.join(suffix));
    }

    /// Verifies roots and every filesystem operation share tilde expansion.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// home/workspace/
    /// create_dir("~/nested/tree")
    /// write, append, read, inspect, copy, rename, and delete under "~/nested"
    /// ```
    ///
    /// # Assertions
    ///
    /// - Root initialization expands `~` before recursive creation.
    /// - Discovery, read, and metadata operations accept expanded paths.
    /// - Directory creation and every file mutation accept expanded paths.
    /// - Copy and rename expand both source and destination paths.
    /// - File and recursive directory deletion accept expanded paths.
    /// - Expanded paths outside the scoped root remain rejected.
    #[test]
    fn roots_and_operations_share_tilde_expansion() {
        let tree = TestRoot::new("tilde-operations");
        let home = tree.path().join("home");
        fs::create_dir(&home).expect("fake home should be created");
        let filesystem = initialize_file_system(
            Path::new("~/workspace"),
            FileSystemOptions::new().create_root(true),
            Some(home.clone()),
        )
        .expect("tilde root should initialize");
        assert_eq!(
            filesystem.root(),
            fs::canonicalize(home.join("workspace"))
                .expect("expanded workspace root should canonicalize")
        );

        filesystem
            .create_dir_blocking(Path::new("~/workspace/nested/tree"))
            .expect("tilde directory path should be created recursively");
        let data_path = Path::new("~/workspace/nested/tree/data.txt");
        filesystem
            .write_file_blocking(data_path, b"hello", WriteMode::Truncate)
            .expect("tilde file path should be written");
        filesystem
            .write_file_blocking(data_path, b" world", WriteMode::Append)
            .expect("tilde file path should be appended");
        assert_eq!(
            filesystem
                .read_file_as_bytes_blocking(data_path)
                .expect("tilde file path should read as bytes"),
            b"hello world"
        );
        assert_eq!(
            filesystem
                .read_file_as_string_blocking(data_path)
                .expect("tilde file path should read as text"),
            "hello world"
        );
        assert_eq!(
            filesystem
                .get_metadata_blocking(data_path)
                .expect("tilde file metadata should load")
                .len(),
            11
        );
        assert_eq!(
            filesystem
                .resolve_existing(data_path, "resolve path")
                .expect("tilde file path should resolve"),
            fs::canonicalize(home.join("workspace/nested/tree/data.txt"))
                .expect("expanded data path should canonicalize")
        );
        assert_eq!(
            filesystem
                .read_dir_blocking(Path::new("~/workspace/nested/tree"))
                .expect("tilde directory path should list")
                .len(),
            1
        );

        filesystem
            .write_and_replace_file_blocking(data_path, b"replacement")
            .expect("tilde file path should be atomically replaced");
        filesystem
            .copy_file_blocking(data_path, Path::new("~/workspace/nested/tree/copy.txt"))
            .expect("tilde transfer paths should copy");
        filesystem
            .rename_blocking(
                Path::new("~/workspace/nested/tree/copy.txt"),
                Path::new("~/workspace/nested/tree/moved.txt"),
            )
            .expect("tilde transfer paths should rename");
        filesystem
            .delete_file_blocking(Path::new("~/workspace/nested/tree/moved.txt"))
            .expect("tilde file path should delete");
        filesystem
            .delete_dir_blocking(Path::new("~/workspace/nested"))
            .expect("tilde directory path should delete recursively");
        assert!(!home.join("workspace/nested").exists());

        fs::write(home.join("outside.txt"), "outside")
            .expect("outside home fixture should be written");
        let error = filesystem
            .resolve_existing(Path::new("~/outside.txt"), "resolve path")
            .expect_err("expanded path outside the scoped root should fail");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }

    /// Verifies contained reads, metadata, and listings preserve generic information.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// root/
    /// ├── Alpha/
    /// ├── notes.txt
    /// └── payload.bin
    /// ```
    ///
    /// # Assertions
    ///
    /// - Listings sort names case-insensitively.
    /// - Entry kinds distinguish directories and files.
    /// - Byte and text reads return complete contents.
    /// - Metadata reports the payload length.
    #[test]
    fn discovery_and_reads_return_contained_file_information() {
        let tree = TestRoot::new("reads");
        fs::create_dir(tree.path().join("Alpha")).expect("directory fixture should be created");
        fs::write(tree.path().join("notes.txt"), "hello").expect("text fixture should be written");
        fs::write(tree.path().join("payload.bin"), [0_u8, 1, 2])
            .expect("binary fixture should be written");
        let filesystem = use_file_system(tree.path()).expect("fixture root should open");

        let entries = filesystem
            .read_dir_blocking(Path::new(""))
            .expect("root should list");
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name().to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["Alpha", "notes.txt", "payload.bin"]
        );
        assert_eq!(entries[0].kind(), FileKind::Directory);
        assert_eq!(entries[1].kind(), FileKind::File);
        assert_eq!(
            filesystem
                .read_file_as_string_blocking(Path::new("notes.txt"))
                .expect("text file should read"),
            "hello"
        );
        assert_eq!(
            filesystem
                .read_file_as_bytes_blocking(Path::new("payload.bin"))
                .expect("binary file should read"),
            [0, 1, 2]
        );
        assert_eq!(
            filesystem
                .get_metadata_blocking(Path::new("payload.bin"))
                .expect("binary metadata should load")
                .len(),
            3
        );
    }

    /// Verifies write, append, atomic replacement, copy, and rename semantics.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// write data.txt -> append -> atomic replace -> copy -> rename
    /// ```
    ///
    /// # Assertions
    ///
    /// - Ordinary writes truncate and appends extend the destination.
    /// - Atomic replacement exposes only the replacement contents afterward.
    /// - Copy returns the byte count and rename moves files and directories.
    /// - Transfers reject existing destinations.
    #[test]
    fn mutations_write_transfer_and_replace_files() {
        let tree = TestRoot::new("mutations");
        let filesystem = use_file_system(tree.path()).expect("fixture root should open");
        filesystem
            .write_file_blocking(Path::new("data.txt"), b"hello", WriteMode::Truncate)
            .expect("file should be written");
        filesystem
            .write_file_blocking(Path::new("data.txt"), b" world", WriteMode::Append)
            .expect("file should be appended");
        assert_eq!(
            fs::read_to_string(tree.path().join("data.txt")).expect("appended file should read"),
            "hello world"
        );

        filesystem
            .write_and_replace_file_blocking(Path::new("data.txt"), b"replacement")
            .expect("file should be replaced");
        let copied = filesystem
            .copy_file_blocking(Path::new("data.txt"), Path::new("copy.txt"))
            .expect("file should copy");
        assert_eq!(copied, 11);
        filesystem
            .rename_blocking(Path::new("copy.txt"), Path::new("moved.txt"))
            .expect("copied file should move");
        assert!(!tree.path().join("copy.txt").exists());
        assert!(tree.path().join("moved.txt").is_file());

        fs::create_dir(tree.path().join("source-dir"))
            .expect("directory rename fixture should be created");
        filesystem
            .rename_blocking(Path::new("source-dir"), Path::new("renamed-dir"))
            .expect("directory should move");
        assert!(!tree.path().join("source-dir").exists());
        assert!(tree.path().join("renamed-dir").is_dir());

        let error = filesystem
            .copy_file_blocking(Path::new("data.txt"), Path::new("moved.txt"))
            .expect_err("existing transfer destination should fail");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    }

    /// Verifies recursive directory operations create ancestors and protect the root.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// root/tree/nested/file
    /// create_dir(tree/nested)
    /// delete_dir(tree)
    /// delete_dir(root)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Recursive creation creates missing ancestor directories.
    /// - Non-empty recursive directory trees can be removed.
    /// - Recursive removal of the scoped root is rejected.
    /// - The root remains present after the rejection.
    #[test]
    fn recursive_directory_operations_create_ancestors_and_protect_root() {
        let tree = TestRoot::new("removal");
        let filesystem = use_file_system(tree.path()).expect("fixture root should open");
        filesystem
            .create_dir_blocking(Path::new("tree/nested"))
            .expect("recursive directory should be created");
        assert!(tree.path().join("tree/nested").is_dir());
        fs::write(tree.path().join("tree/nested/file"), "data")
            .expect("nested file should be written");
        filesystem
            .delete_dir_blocking(Path::new("tree"))
            .expect("recursive directory should be removed");

        let error = filesystem
            .delete_dir_blocking(Path::new(""))
            .expect_err("root removal should fail");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(tree.path().is_dir());
    }

    /// Verifies recursive directory deletion does not traverse symbolic links.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// root/link -> outside/target/
    /// delete_dir(link)
    /// ```
    ///
    /// # Assertions
    ///
    /// - The symbolic link is removed.
    /// - The outside target and its contents remain present.
    #[cfg(unix)]
    #[test]
    fn recursive_directory_deletion_does_not_traverse_symlinks() {
        use std::os::unix::fs::symlink;

        let tree = TestRoot::new("delete-symlink");
        let outside = TestRoot::new("delete-symlink-outside");
        fs::write(outside.path().join("kept.txt"), "keep")
            .expect("outside fixture should be written");
        symlink(outside.path(), tree.path().join("link"))
            .expect("directory symlink fixture should be created");
        let filesystem = use_file_system(tree.path()).expect("fixture root should open");

        filesystem
            .delete_dir_blocking(Path::new("link"))
            .expect("directory symlink should be removed");

        assert!(fs::symlink_metadata(tree.path().join("link")).is_err());
        assert!(outside.path().join("kept.txt").is_file());
    }

    /// Verifies parent traversal and outside absolute paths are rejected.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// ../outside
    /// /another-temporary-root
    /// ```
    ///
    /// # Assertions
    ///
    /// - An explicit parent component fails with `PermissionDenied`.
    /// - An existing absolute path outside the root fails with `PermissionDenied`.
    #[test]
    fn containment_rejects_parent_and_absolute_escape_paths() {
        let tree = TestRoot::new("contained");
        let outside = TestRoot::new("outside");
        let filesystem = use_file_system(tree.path()).expect("fixture root should open");

        let parent_error = filesystem
            .resolve_existing(Path::new("../outside"), "resolve path")
            .expect_err("parent traversal should fail");
        assert_eq!(parent_error.kind(), io::ErrorKind::PermissionDenied);
        let outside_error = filesystem
            .resolve_existing(outside.path(), "resolve path")
            .expect_err("outside absolute path should fail");
        assert_eq!(outside_error.kind(), io::ErrorKind::PermissionDenied);
    }

    /// Verifies a public read operation starts immediately and retains its result.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// read_file_as_string("document.txt")
    /// ```
    ///
    /// # Assertions
    ///
    /// - Calling the method starts the operation and marks it pending.
    /// - Completion increments the action version.
    /// - The retained output contains the file text.
    #[tokio::test(flavor = "current_thread")]
    async fn read_file_as_string_completes_with_reactive_state() {
        let tree = TestRoot::new("action");
        fs::write(tree.path().join("document.txt"), "ready")
            .expect("action fixture should be written");
        let filesystem = use_file_system(tree.path()).expect("fixture root should open");
        let owner = Owner::new();
        let action = owner.with(|| filesystem.read_file_as_string("document.txt"));

        assert!(action.is_pending_untracked());
        timeout(Duration::from_secs(2), async {
            while action.version().get_untracked() == 0 {
                yield_now().await;
            }
        })
        .await
        .expect("filesystem operation should complete");

        assert!(!action.is_pending_untracked());
        action.value().with_untracked(|result| {
            assert_eq!(
                result
                    .as_ref()
                    .expect("action should retain a result")
                    .as_ref()
                    .expect("read should succeed"),
                "ready"
            );
        });
    }

    /// Verifies a retained operation can retry its captured arguments.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// let append = filesystem.append_file("events.log", b"event\n");
    /// append.dispatch(());
    /// ```
    ///
    /// # Assertions
    ///
    /// - Calling `append_file` performs the first append immediately.
    /// - Dispatching `()` retries the same path and contents.
    /// - The completion version counts both successful runs.
    #[tokio::test(flavor = "current_thread")]
    async fn retained_operation_retries_captured_arguments() {
        let tree = TestRoot::new("operation-retry");
        let filesystem = use_file_system(tree.path()).expect("fixture root should open");
        let owner = Owner::new();
        let operation = owner.with(|| filesystem.append_file("events.log", b"event\n"));

        timeout(Duration::from_secs(2), async {
            while operation.version().get_untracked() < 1 {
                yield_now().await;
            }
        })
        .await
        .expect("initial append should complete");
        operation.dispatch(());
        timeout(Duration::from_secs(2), async {
            while operation.version().get_untracked() < 2 {
                yield_now().await;
            }
        })
        .await
        .expect("retried append should complete");

        assert_eq!(
            fs::read_to_string(tree.path().join("events.log"))
                .expect("appended fixture should read"),
            "event\nevent\n"
        );
    }

    /// Verifies listings hide broken and escaping symbolic links.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// root/inside-link -> root/inside.txt
    /// root/outside-link -> outside/outside.txt
    /// root/broken-link -> missing.txt
    /// ```
    ///
    /// # Assertions
    ///
    /// - The contained symlink is returned and marked as a symlink.
    /// - The escaping and broken symlinks are omitted.
    #[cfg(unix)]
    #[test]
    fn listings_follow_only_contained_symlink_targets() {
        use std::os::unix::fs::symlink;

        let tree = TestRoot::new("symlinks");
        let outside = TestRoot::new("symlink-outside");
        fs::write(tree.path().join("inside.txt"), "inside")
            .expect("inside fixture should be written");
        fs::write(outside.path().join("outside.txt"), "outside")
            .expect("outside fixture should be written");
        symlink("inside.txt", tree.path().join("inside-link"))
            .expect("contained symlink should be created");
        symlink(
            outside.path().join("outside.txt"),
            tree.path().join("outside-link"),
        )
        .expect("escaping symlink should be created");
        symlink("missing.txt", tree.path().join("broken-link"))
            .expect("broken symlink should be created");
        let filesystem = use_file_system(tree.path()).expect("fixture root should open");

        let entries = filesystem
            .read_dir_blocking(Path::new(""))
            .expect("root should list");
        assert!(
            entries
                .iter()
                .any(|entry| { entry.name() == OsStr::new("inside-link") && entry.is_symlink() })
        );
        assert!(!entries.iter().any(|entry| {
            matches!(entry.name().to_str(), Some("outside-link" | "broken-link"))
        }));
    }
}
