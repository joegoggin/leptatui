//! Style conversion tests.
//!
//! These tests cover conversion from Leptatui style helpers into Ratatui style,
//! padding, and block values.

use leptatui::{
    BorderType, Borders, Color, LayoutDirection, MediaQuery, Modifier, StyleDeclarations,
    StyleMetadata, StyleModule, StyleSelector, StyleValue, Stylesheet, ThemeValue, ThemeVariables,
    TuiSize, TuiSpacing, TuiStyle, ViewType, ViewportSize, button, image, stylesheet, text,
    theme_color,
};
use ratatui::{style::Style, widgets::Padding};

include!("cascade/mod.rs");
include!("foundation.rs");
include!("macro_integration.rs");
include!("modules.rs");
include!("theme.rs");
