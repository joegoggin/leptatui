//! Components owned by the Viewer page.
//!
//! # Modules
//!
//! - [`document`] — Path-backed Markdown content and editor diagnostics.

mod document;

pub(super) use document::{ViewerDocument, ViewerDocumentProps};
