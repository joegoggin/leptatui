//! Style conversion tests.
//!
//! These tests cover conversion from Leptatui style helpers into Ratatui style,
//! padding, and block values.

use leptatui::{
    AlignContent, AlignItems, AlignSelf, Axes, BorderType, Borders, BoxSizing, Color, Dimension,
    Display, Edges, FlexDirection, FlexWrap, GridAutoFlow, GridLine, GridPlacement, JustifyContent,
    JustifyItems, JustifySelf, LayoutDirection, LayoutSize, Length, LengthAuto, MediaQuery,
    Modifier, Overflow, Position, StyleDeclarations, StyleMetadata, StyleModule, StyleSelector,
    StyleValue, Stylesheet, ThemeValue, ThemeVariables, TuiSize, TuiSpacing, TuiStyle, ViewType,
    ViewportSize, ZIndex, button, image, stylesheet, text, theme_color,
};
use ratatui::{style::Style, widgets::Padding};

include!("cascade/mod.rs");
include!("foundation.rs");
include!("layout_values.rs");
include!("macro_integration.rs");
include!("modules.rs");
include!("theme.rs");
