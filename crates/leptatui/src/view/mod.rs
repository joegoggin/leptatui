//! Basic renderable terminal views.
//!
//! This module provides a small view tree for simple hand-written interfaces
//! and builder functions for creating common view variants.

mod builders;
mod component_view;
mod dynamic;
mod metadata;
mod model;
mod render;

pub(crate) use builders::component_factory;
pub use builders::{
    block, button, column, component, dynamic, form, image, input, row, text, text_area,
};
pub use metadata::{EditableState, StyleMetadata, ViewType, VimMode};
pub use model::{ButtonAction, FormAction, ImageSource, InputAction, View};
