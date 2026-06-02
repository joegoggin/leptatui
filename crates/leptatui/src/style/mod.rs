//! Styling primitives for Leptatui applications.
//!
//! This module wraps common Ratatui style, border, and padding configuration in
//! small builder-style value types.

pub use ratatui::{
    style::{Color, Modifier},
    widgets::{BorderType, Borders},
};

mod spacing;
mod tui_style;

pub use spacing::TuiSpacing;
pub use tui_style::TuiStyle;
