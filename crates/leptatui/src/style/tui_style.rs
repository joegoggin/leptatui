//! Builder-style terminal UI style values.
//!
//! This module collects text colors, modifiers, borders, padding, and layout
//! direction before converting supported visual values into Ratatui values.

use ratatui::{style::Style, widgets::Block};

use super::{BorderType, Borders, Color, LayoutDirection, Modifier, TuiSpacing};

/// Reusable style values for terminal UI elements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TuiStyle {
    /// Text foreground color.
    pub foreground: Option<Color>,
    /// Text background color.
    pub background: Option<Color>,
    /// Text modifiers such as bold or italic.
    pub modifiers: Option<Modifier>,
    /// Widget border sides.
    pub borders: Option<Borders>,
    /// Widget border glyph set.
    pub border_type: Option<BorderType>,
    /// Internal widget padding.
    pub padding: Option<TuiSpacing>,
    /// Optional child layout direction override.
    pub direction: Option<LayoutDirection>,
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
            modifiers: None,
            borders: None,
            border_type: None,
            padding: None,
            direction: None,
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
        self.modifiers = Some(self.modifiers.unwrap_or(Modifier::empty()) | modifier);
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
        self.borders = Some(borders);
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
        self.border_type = Some(border_type);
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
        self.padding = Some(padding);
        self
    }

    /// Sets the child layout direction.
    ///
    /// # Arguments
    ///
    /// * `direction` — Direction used to split child view areas.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with the provided layout direction.
    pub const fn direction(mut self, direction: LayoutDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Returns the style values inherited by descendant views.
    ///
    /// Foreground color and text modifiers inherit across view boundaries.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing inheritable values from this style.
    pub(crate) const fn inherited_values(self) -> Self {
        Self {
            foreground: self.foreground,
            background: None,
            modifiers: self.modifiers,
            borders: None,
            border_type: None,
            padding: None,
            direction: None,
        }
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

        if let Some(modifiers) = self.modifiers
            && !modifiers.is_empty()
        {
            style = style.add_modifier(modifiers);
        }

        style
    }

    /// Converts this style to a Ratatui block.
    ///
    /// # Returns
    ///
    /// A [`Block`] value containing configured style, borders, and padding.
    pub fn to_block(self) -> Block<'static> {
        self.to_block_with_default_borders(Borders::NONE)
    }

    /// Converts this style to a Ratatui block with fallback borders.
    ///
    /// # Arguments
    ///
    /// * `default_borders` — Border sides to use when this style does not
    ///   configure borders explicitly.
    ///
    /// # Returns
    ///
    /// A [`Block`] value containing configured style, borders, and padding.
    pub(crate) fn to_block_with_default_borders(self, default_borders: Borders) -> Block<'static> {
        Block::new()
            .style(self.to_ratatui_style())
            .borders(self.borders.unwrap_or(default_borders))
            .border_type(self.border_type.unwrap_or(BorderType::Plain))
            .padding(self.padding.unwrap_or(TuiSpacing::ZERO).into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherited_values_keep_text_style_and_drop_surface_style() {
        let inherited = TuiStyle::new()
            .foreground(Color::Green)
            .background(Color::Blue)
            .modifier(Modifier::BOLD)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(TuiSpacing::uniform(1))
            .direction(LayoutDirection::Column)
            .inherited_values();

        assert_eq!(inherited.foreground, Some(Color::Green));
        assert_eq!(inherited.modifiers, Some(Modifier::BOLD));
        assert_eq!(inherited.background, None);
        assert_eq!(inherited.borders, None);
        assert_eq!(inherited.border_type, None);
        assert_eq!(inherited.padding, None);
        assert_eq!(inherited.direction, None);
    }
}
