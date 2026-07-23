//! Link targets and linked rich-text storage.
//!
//! This module classifies URL, filesystem, and fragment targets and retains
//! focusable link ranges inside semantic rich text. Standalone and embedded
//! links share the same target resolution and system-opening behavior.
//!
//! # Modules
//!
//! - [`geometry`] — Link-aware wrapping and rendered hit-test geometry.
//! - [`rich_text`] — Rich-text content, embedded links, focus, and styling.
//! - [`target`] — Link target classification, resolution, and activation.

mod geometry;
mod rich_text;
mod target;

pub(crate) use geometry::RichTextWrapMode;
pub use rich_text::RichText;
pub(crate) use rich_text::{InlineLink, LinkedSpan, impl_rich_text_view, resolved_rich_text};
pub use target::LinkTarget;
pub(crate) use target::open_link_target;
