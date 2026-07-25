//! Transient computed-layout integration.
//!
//! Each root render mirrors visible styleable views into a short-lived
//! Taffy tree, delegates leaf sizing to [`View::measure`](crate::View::measure),
//! and stores rounded engine-independent rectangles on view metadata.
//!
//! # Modules
//!
//! - [`geometry`] — Retained terminal geometry conversion and assignment.
//! - [`measure`] — View measurement and logical-path traversal.
//! - [`style`] — Leptatui-to-Taffy style conversion.
//! - [`tree`] — Layout-tree construction, computation, and orchestration.

mod geometry;
mod measure;
mod style;
mod tree;

pub(crate) use tree::prepare_layout;

/// Logical child indexes from the rendered root to one layout box.
#[derive(Clone, Debug)]
struct LayoutPath(
    /// Ordered logical child indexes.
    Vec<usize>,
);
