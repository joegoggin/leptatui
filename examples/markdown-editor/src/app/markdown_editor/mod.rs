//! Routed Markdown editor shell and its co-located stylesheet.
//!
//! # Modules
//!
//! - [`component`] — Routed application shell and global controls.
//! - [`style`] — Markdown editor shell stylesheet registration.

mod component;
mod style;

pub(in crate::app) use component::{MarkdownEditor, MarkdownEditorProps};
