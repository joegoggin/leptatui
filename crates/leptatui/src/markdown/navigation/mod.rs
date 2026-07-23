//! File-backed Markdown navigation, link resolution, and page history.
//!
//! Parsing support classifies document-local targets and assigns heading
//! anchors, while the runtime view retains loaded pages and navigation history.
//!
//! # Modules
//!
//! - [`parse`] — Parse-time link resolution and heading-anchor generation.
//! - [`view`] — File-backed Markdown view state and page history.

mod parse;
mod view;

pub(in crate::markdown) use parse::MarkdownParseContext;
pub use view::MarkdownView;
