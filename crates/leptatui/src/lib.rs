//! Public runtime crate for Leptatui.
//!
//! Leptatui combines Leptos reactive primitives with Ratatui rendering helpers
//! and a managed Crossterm terminal app loop.
//!
//! # Modules
//!
//! - [`app`] — Terminal setup, event polling, and app-loop runtime APIs.
//! - [`component`] — Component rendering contracts and frame contexts.
//! - [`context`] — Leptos context re-exports for terminal applications.
//! - [`node`] — Basic renderable node builders for hand-written terminal UI.
//! - [`prelude`] — Common imports for application code.
//! - [`style`] — Styling and spacing helpers built on Ratatui types.

pub mod app;
pub mod component;
pub mod context;
pub mod node;
pub mod prelude;
pub mod style;

pub use app::{App, AppControl, AppRoot, Error, Result};
pub use component::{Component, RenderCtx};
pub use node::{Node, block, button, column, row, text};
pub use style::{BorderType, Borders, Color, Modifier, TuiSpacing, TuiStyle};
