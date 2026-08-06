//! Infrastructure services used by the Markdown editor.
//!
//! # Modules
//!
//! - [`filesystem`] — Volume boundaries and Markdown path validation.
//! - [`recent_files`] — Persistent recent-file storage.

mod filesystem;
mod recent_files;

pub(crate) use filesystem::is_markdown_path;
#[cfg(test)]
pub(crate) use recent_files::RECENT_FILE_LIMIT;
pub(crate) use recent_files::RecentFilesStore;
