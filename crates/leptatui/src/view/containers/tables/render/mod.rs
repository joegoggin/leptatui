//! Semantic table layout, grid construction, and rendering.
//!
//! # Modules
//!
//! - [`grid`] — Border glyphs and cell-alignment conversion.
//! - [`layout`] — Semantic collection, responsive sizing, and cell wrapping.
//! - [`view`] — Terminal drawing and intrinsic height calculation.

mod grid;
mod layout;
mod view;

pub(super) use view::{min_height_for_table_view, render_table_view};
