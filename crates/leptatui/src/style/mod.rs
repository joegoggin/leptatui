//! Styling primitives for Leptatui applications.
//!
//! This module wraps common Ratatui style, border, and padding configuration in
//! small builder-style value types.
//!
//! # Modules
//!
//! - `cascade` — Selectors, declarations, media queries, and stylesheets.
//! - `foundation` — Layout, size, spacing, theme, and resolved style values.
//! - `layout` — Engine-independent web layout values.
//! - `style_module` — Named collections of stylesheet and theme values.

pub use ratatui::{
    style::{Color, Modifier},
    widgets::{BorderType, Borders},
};

mod cascade;
mod foundation;
mod layout;
mod style_module;

pub use cascade::{
    MediaQuery, StyleDeclarations, StyleRule, StyleSelector, Stylesheet, ViewportSize,
};
pub use foundation::{
    LayoutDirection, ThemeValue, ThemeVariables, TuiSize, TuiSpacing, TuiStyle, theme_color,
};
pub use layout::{
    AlignContent, AlignItems, AlignSelf, Axes, BoxSizing, Dimension, Display, Edges, FlexDirection,
    FlexWrap, Fraction, GridAutoFlow, GridLine, GridPlacement, JustifyContent, JustifyItems,
    JustifySelf, LayoutSize, Length, LengthAuto, Overflow, Position, ZIndex,
};
pub use style_module::{StyleModule, StyleValue};
