//! Application hooks for shared domain contexts.
//!
//! # Modules
//!
//! - [`mod@use_files`] — Shared file signals and their required context hook.
//! - [`mod@use_workspace`] — Workspace resources and their required context hook.

mod use_files;
mod use_workspace;

pub(crate) use use_files::{EditorFailure, Files, use_files};
pub(crate) use use_workspace::{WorkspaceContext, use_workspace};
