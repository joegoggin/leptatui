//! Styling primitives for Leptatui applications.
//!
//! This module wraps common Ratatui style, border, and padding configuration in
//! small builder-style value types.
//!
//! # Modules
//!
//! - `cascade` - Selectors, declarations, media queries, and stylesheets.
//! - `foundation` - Layout, size, spacing, theme, and resolved style values.
//! - `style_module` - Named collections of stylesheet and theme values.

pub use ratatui::{
    style::{Color, Modifier},
    widgets::{BorderType, Borders},
};

mod cascade;
mod foundation;
mod style_module;

pub use cascade::{
    MediaQuery, StyleDeclarations, StyleRule, StyleSelector, Stylesheet, ViewportSize,
};
pub use foundation::{
    LayoutDirection, ThemeValue, ThemeVariables, TuiSize, TuiSpacing, TuiStyle, theme_color,
};
pub use style_module::{StyleModule, StyleValue};
