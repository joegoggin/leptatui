//! Foundational style values used by declarations and resolved terminal styles.
//!
//! # Modules
//!
//! - [`layout`] — Layout direction values.
//! - [`size`] — Terminal width and height constraints.
//! - [`spacing`] — Padding and gap values.
//! - [`theme`] — Theme variables and color lookup.
//! - [`tui_style`] — Fully resolved terminal styles.

mod layout;
mod size;
mod spacing;
mod theme;
mod tui_style;

pub use layout::LayoutDirection;
pub use size::TuiSize;
pub use spacing::TuiSpacing;
pub use theme::{ThemeValue, ThemeVariables, theme_color};
pub use tui_style::TuiStyle;
