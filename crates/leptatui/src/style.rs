//! Styling primitives for Leptatui applications.
//!
//! This module wraps common Ratatui style, border, and padding configuration in
//! small builder-style value types.

pub use ratatui::{
    style::{Color, Modifier},
    widgets::{BorderType, Borders},
};

use ratatui::{
    style::Style,
    widgets::{Block, Padding},
};

/// Simple terminal-cell spacing values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TuiSpacing {
    /// Cells to reserve on the left.
    pub left: u16,
    /// Cells to reserve on the right.
    pub right: u16,
    /// Cells to reserve above the content.
    pub top: u16,
    /// Cells to reserve below the content.
    pub bottom: u16,
}

impl TuiSpacing {
    /// No spacing.
    pub const ZERO: Self = Self {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
    };

    /// Creates spacing with every side specified.
    ///
    /// # Arguments
    ///
    /// * `left` — Cells to reserve on the left.
    /// * `right` — Cells to reserve on the right.
    /// * `top` — Cells to reserve above the content.
    /// * `bottom` — Cells to reserve below the content.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value with each side set independently.
    pub const fn new(left: u16, right: u16, top: u16, bottom: u16) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Creates equal spacing on every side.
    ///
    /// # Arguments
    ///
    /// * `value` — Cells to reserve on each side.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value with all sides set to `value`.
    pub const fn uniform(value: u16) -> Self {
        Self::new(value, value, value, value)
    }

    /// Creates equal horizontal spacing.
    ///
    /// # Arguments
    ///
    /// * `value` — Cells to reserve on the left and right sides.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value with horizontal sides set to `value`.
    pub const fn horizontal(value: u16) -> Self {
        Self::new(value, value, 0, 0)
    }

    /// Creates equal vertical spacing.
    ///
    /// # Arguments
    ///
    /// * `value` — Cells to reserve on the top and bottom sides.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value with vertical sides set to `value`.
    pub const fn vertical(value: u16) -> Self {
        Self::new(0, 0, value, value)
    }
}

impl From<TuiSpacing> for Padding {
    /// Converts terminal-cell spacing into Ratatui padding.
    ///
    /// # Arguments
    ///
    /// * `spacing` — Leptatui spacing value to convert.
    ///
    /// # Returns
    ///
    /// A [`Padding`] value with matching side sizes.
    fn from(spacing: TuiSpacing) -> Self {
        Self::new(spacing.left, spacing.right, spacing.top, spacing.bottom)
    }
}

/// Reusable style values for terminal UI elements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TuiStyle {
    /// Text foreground color.
    pub foreground: Option<Color>,
    /// Text background color.
    pub background: Option<Color>,
    /// Text modifiers such as bold or italic.
    pub modifiers: Modifier,
    /// Widget border sides.
    pub borders: Borders,
    /// Widget border glyph set.
    pub border_type: BorderType,
    /// Internal widget padding.
    pub padding: TuiSpacing,
}

impl Default for TuiStyle {
    /// Creates an empty terminal UI style.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with no colors, modifiers, borders, or padding.
    fn default() -> Self {
        Self::new()
    }
}

impl TuiStyle {
    /// Creates an empty style.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with no colors, modifiers, borders, or padding.
    pub const fn new() -> Self {
        Self {
            foreground: None,
            background: None,
            modifiers: Modifier::empty(),
            borders: Borders::NONE,
            border_type: BorderType::Plain,
            padding: TuiSpacing::ZERO,
        }
    }

    /// Sets the foreground color.
    ///
    /// # Arguments
    ///
    /// * `color` — Text foreground color to apply.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with `color` stored as the foreground.
    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    /// Sets the background color.
    ///
    /// # Arguments
    ///
    /// * `color` — Text background color to apply.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with `color` stored as the background.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Adds one or more text modifiers.
    ///
    /// # Arguments
    ///
    /// * `modifier` — Text modifier flags to add.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with `modifier` added to the current modifiers.
    pub fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifiers |= modifier;
        self
    }

    /// Sets the visible borders.
    ///
    /// # Arguments
    ///
    /// * `borders` — Border sides to render.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with the provided border sides.
    pub const fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    /// Sets the border glyph style.
    ///
    /// # Arguments
    ///
    /// * `border_type` — Ratatui border glyph set to use.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with the provided border glyph style.
    pub const fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_type = border_type;
        self
    }

    /// Sets internal padding.
    ///
    /// # Arguments
    ///
    /// * `padding` — Internal padding to apply to block widgets.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with the provided padding.
    pub const fn padding(mut self, padding: TuiSpacing) -> Self {
        self.padding = padding;
        self
    }

    /// Converts this style to Ratatui's text and cell style.
    ///
    /// # Returns
    ///
    /// A [`Style`] value containing configured colors and modifiers.
    pub fn to_ratatui_style(self) -> Style {
        let mut style = Style::new();

        if let Some(color) = self.foreground {
            style = style.fg(color);
        }

        if let Some(color) = self.background {
            style = style.bg(color);
        }

        if !self.modifiers.is_empty() {
            style = style.add_modifier(self.modifiers);
        }

        style
    }

    /// Converts this style to a Ratatui block.
    ///
    /// # Returns
    ///
    /// A [`Block`] value containing configured style, borders, and padding.
    pub fn to_block(self) -> Block<'static> {
        Block::new()
            .style(self.to_ratatui_style())
            .borders(self.borders)
            .border_type(self.border_type)
            .padding(self.padding.into())
    }
}

impl From<TuiStyle> for Style {
    /// Converts a Leptatui style into a Ratatui style.
    ///
    /// # Arguments
    ///
    /// * `style` — Leptatui style to convert.
    ///
    /// # Returns
    ///
    /// A [`Style`] value containing configured colors and modifiers.
    fn from(style: TuiStyle) -> Self {
        style.to_ratatui_style()
    }
}
