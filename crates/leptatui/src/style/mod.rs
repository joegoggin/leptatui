//! Styling primitives for Leptatui applications.
//!
//! This module wraps Ratatui presentation values and exposes the
//! engine-independent, web-inspired layout vocabulary shared by inline
//! [`TuiStyle`] values and [`macro@crate::stylesheet`] declarations. Terminal
//! cells are the absolute unit; containing-block percentages, viewport units,
//! automatic values, intrinsic grid tracks, and fractions remain typed until
//! layout.
//!
//! # Layout Example
//!
//! ```
//! use leptatui::prelude::*;
//!
//! let inline = TuiStyle::new()
//!     .display(Display::Flex)
//!     .size(LayoutSize::new(
//!         Dimension::from(Length::percent(100.0)),
//!         Dimension::from(Length::vh(50.0)),
//!     ))
//!     .gap(Axes::all(Length::cells(1.0)))
//!     .overflow(Axes::new(Overflow::Hidden, Overflow::Auto));
//!
//! let panel = div((text("Main"), text("Sidebar"))).with_inline_style(inline);
//! let _ = panel;
//! ```
//!
//! General [`Dimension::MinContent`] and [`Dimension::MaxContent`] values
//! currently behave like [`Dimension::Auto`], while
//! [`Dimension::FitContent`] behaves like its contained length. Grid track
//! types support automatic, min-content, max-content, fractional, `minmax`,
//! and repeated sizing directly. Computed floating-point geometry is retained
//! as terminal-cell rectangles with cumulative rounding across sibling
//! sequences.
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
pub use foundation::{ThemeValue, ThemeVariables, TuiSize, TuiSpacing, TuiStyle, theme_color};
pub use layout::{
    AlignContent, AlignItems, AlignSelf, Axes, BoxSizing, Dimension, Display, Edges, FlexDirection,
    FlexWrap, Fraction, GridAutoFlow, GridLine, GridMaxTrackSize, GridMinTrackSize, GridPlacement,
    GridRepeat, GridTemplateTrack, GridTrackSize, JustifyContent, JustifyItems, JustifySelf,
    LayoutSize, Length, LengthAuto, Overflow, Position, ZIndex,
};
pub use style_module::{StyleModule, StyleValue};
