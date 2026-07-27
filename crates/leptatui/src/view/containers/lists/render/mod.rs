//! Semantic-list rendering, measurement, and focus geometry.
//!
//! # Modules
//!
//! - [`focus`] — Focused-descendant traversal and vertical spans.
//! - [`layout`] — Marker creation, indentation, and horizontal geometry.
//! - [`measure`] — Two-axis list and list-item measurement.
//! - [`view`] — Terminal painting for lists, markers, and item content.

mod focus;
mod layout;
mod measure;
mod view;

pub(crate) use focus::focused_control_span_for_list_view;
pub(crate) use measure::measure_list_view;
pub(crate) use view::render_list_view;
