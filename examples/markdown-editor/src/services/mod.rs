//! Infrastructure services used by the Markdown editor.
//!
//! # Modules
//!
//! - [`editor_process`] — External editor configuration and process launching.
//! - [`filesystem`] — Anchored workspace and Markdown path access.
//! - [`recent_files`] — Persistent recent-file storage.

mod editor_process;
mod filesystem;
mod recent_files;

pub(crate) use editor_process::EditorProcess;
#[cfg(test)]
pub(crate) use editor_process::{EnvironmentReader, ProcessLauncher};
pub(crate) use filesystem::FileSystem;
pub(crate) use recent_files::RecentFilesStore;
