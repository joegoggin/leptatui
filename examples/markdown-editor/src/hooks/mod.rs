//! Application hooks for shared reactive values.
//!
//! # Modules
//!
//! - [`mod@use_files`] — Shared file signals and their required context hook.

mod use_files;

pub(crate) use use_files::{EditorFailure, Files, use_files};
