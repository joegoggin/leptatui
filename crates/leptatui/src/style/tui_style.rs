use ratatui::{style::Style, widgets::Block};

use super::{BorderType, Borders, Color, Modifier, TuiSpacing};

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
