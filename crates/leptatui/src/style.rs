//! Styling primitives for Leptatui applications.

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

    /// Create spacing with every side specified.
    pub const fn new(left: u16, right: u16, top: u16, bottom: u16) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Create equal spacing on every side.
    pub const fn uniform(value: u16) -> Self {
        Self::new(value, value, value, value)
    }

    /// Create equal horizontal spacing.
    pub const fn horizontal(value: u16) -> Self {
        Self::new(value, value, 0, 0)
    }

    /// Create equal vertical spacing.
    pub const fn vertical(value: u16) -> Self {
        Self::new(0, 0, value, value)
    }
}

impl From<TuiSpacing> for Padding {
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
    fn default() -> Self {
        Self::new()
    }
}

impl TuiStyle {
    /// Create an empty style.
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

    /// Set the foreground color.
    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Add one or more text modifiers.
    pub fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifiers |= modifier;
        self
    }

    /// Set the visible borders.
    pub const fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    /// Set the border glyph style.
    pub const fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_type = border_type;
        self
    }

    /// Set internal padding.
    pub const fn padding(mut self, padding: TuiSpacing) -> Self {
        self.padding = padding;
        self
    }

    /// Convert this style to Ratatui's text/cell style.
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

    /// Convert this style to a Ratatui block.
    pub fn to_block(self) -> Block<'static> {
        Block::new()
            .style(self.to_ratatui_style())
            .borders(self.borders)
            .border_type(self.border_type)
            .padding(self.padding.into())
    }
}

impl From<TuiStyle> for Style {
    fn from(style: TuiStyle) -> Self {
        style.to_ratatui_style()
    }
}
