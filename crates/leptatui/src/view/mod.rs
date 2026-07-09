//! Basic renderable terminal views.
//!
//! This module provides a small view tree for hand-written interfaces, builder
//! functions for common and semantic view variants, and Ratatui rich-text
//! types used by headings and paragraphs.

mod builders;
mod component_view;
mod dynamic;
mod metadata;
mod model;
mod render;

pub(crate) use builders::component_factory;
pub use builders::{
    block, button, column, component, dynamic, form, h1, h2, h3, h4, h5, h6, image, input,
    paragraph, progress_bar, row, text, text_area,
};
pub use metadata::{EditableState, StyleMetadata, ViewType, VimMode};
pub use model::{ButtonAction, FormAction, ImageSource, InputAction, View};
pub use ratatui::text::{Line, Span, Text};
