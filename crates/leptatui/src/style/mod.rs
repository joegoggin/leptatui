//! Styling primitives for Leptatui applications.
//!
//! This module wraps common Ratatui style, border, and padding configuration in
//! small builder-style value types.

pub use ratatui::{
    style::{Color, Modifier},
    widgets::{BorderType, Borders},
};

mod declarations;
mod selector;
mod spacing;
mod stylesheet;
mod theme;
mod tui_style;

pub use declarations::StyleDeclarations;
pub use selector::StyleSelector;
pub use spacing::TuiSpacing;
pub use stylesheet::{StyleRule, Stylesheet};
pub use theme::{ThemeValue, ThemeVariables, theme_color};
pub use tui_style::TuiStyle;
