//! Infrastructure services used by the Markdown editor.
//!
//! # Modules
//!
//! - [`editor_process`] — External editor configuration and process launching.
//! - [`editor_session`] — Managed-terminal coordination for editor processes.
//! - [`filesystem`] — Volume boundaries and Markdown path validation.
//! - [`recent_files`] — Persistent recent-file storage.

mod editor_process;
mod editor_session;
mod filesystem;
mod recent_files;

pub(crate) use editor_process::EditorProcess;
#[cfg(test)]
pub(crate) use editor_process::{EnvironmentReader, ProcessLauncher};
pub(crate) use editor_session::EditorSession;
pub(crate) use filesystem::{is_markdown_path, volume_root};
#[cfg(test)]
pub(crate) use recent_files::RECENT_FILE_LIMIT;
pub(crate) use recent_files::RecentFilesStore;
