//! Public runtime crate for Leptatui.
//!
//! Leptatui combines Leptos reactive primitives with Ratatui rendering helpers
//! and a managed Crossterm terminal app loop.
//!
//! # Modules
//!
//! - [`app`] — Terminal setup, event polling, and app-loop runtime APIs.
//! - [`mod@component`] — Component rendering contracts and frame contexts.
//! - [`context`] — Typed render-scope context APIs with Leptos owner fallback.
//! - [`node`] — Basic renderable node builders for hand-written terminal UI.
//! - [`prelude`] — Common imports for application code.
//! - [`style`] — Styling and spacing helpers built on Ratatui types.

pub mod app;
pub mod component;
pub mod context;
pub mod node;
pub mod prelude;
pub mod style;

extern crate self as leptatui;

pub use app::{App, AppControl, AppRoot, Error, Result};
pub use component::{Component, RenderCtx};
pub use leptatui_macros::{component, view};
pub use node::{
    Node, NodeType, StyleMetadata, block, button, column, component, dynamic, row, text,
};
pub use style::{BorderType, Borders, Color, Modifier, TuiSpacing, TuiStyle};
