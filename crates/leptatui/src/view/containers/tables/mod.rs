//! Semantic table views and shared table rendering.
//!
//! # Modules
//!
//! - [`render`] — Responsive layout, grid construction, and drawing.
//! - [`table`] — Top-level semantic table container.
//! - [`table_cell`] — Inline table cell values and alignment.
//! - [`table_row`] — Table row containers.
//! - [`table_section`] — Header and body section containers.

pub(crate) mod render;
pub(crate) mod table;
pub(crate) mod table_cell;
pub(crate) mod table_row;
pub(crate) mod table_section;
