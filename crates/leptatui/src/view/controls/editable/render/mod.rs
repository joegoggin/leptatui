//! Rendering orchestration, content preparation, and viewport geometry.
//!
//! # Modules
//!
//! - [`content`] — Paragraph and pending-insert display construction.
//! - [`geometry`] — Scrolling, wrapping, terminal cursor, and focus geometry.
//! - [`view`] — Editable-control rendering and intrinsic measurement.

mod content;
mod geometry;
mod view;

pub(crate) use geometry::focused_control_span_for_editor;
pub(crate) use view::{measure_editable_text_view, render_editable_text_view};
