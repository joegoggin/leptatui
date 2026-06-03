//! Basic renderable terminal nodes.
//!
//! This module provides a small node tree for simple hand-written interfaces
//! and builder functions for creating common node variants.

mod builders;
mod component_node;
mod dynamic;
mod metadata;
mod model;
mod render;

pub use builders::{block, button, column, component, dynamic, row, text};
pub use metadata::{NodeType, StyleMetadata};
pub use model::{ButtonAction, Node};
