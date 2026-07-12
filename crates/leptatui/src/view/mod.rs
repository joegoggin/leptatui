//! Basic renderable terminal views.
//!
//! This module provides a small view tree for hand-written interfaces, builder
//! functions for common and semantic view variants, Ratatui rich-text types
//! used by headings and paragraphs, and nested block-oriented list views.

mod builders;
mod component_view;
mod dynamic;
mod metadata;
mod model;
mod render;

pub(crate) use builders::component_factory;
pub use builders::{
    block, button, column, component, dynamic, form, h1, h2, h3, h4, h5, h6, image, input,
    list_item, ordered_list, paragraph, progress_bar, row, text, text_area, unordered_list,
};
pub use metadata::{EditableState, StyleMetadata, ViewType, VimMode};
pub use model::{ButtonAction, FormAction, ImageSource, InputAction, View};
pub use ratatui::text::{Line, Span, Text};
