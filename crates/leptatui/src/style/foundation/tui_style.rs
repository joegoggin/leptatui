//! Builder-style terminal UI style values.
//!
//! This module collects visual styling and authored layout properties before
//! converting the Ratatui-supported subset into terminal rendering values.

use ratatui::{style::Style, widgets::Block};

use crate::style::{
    AlignContent, AlignItems, AlignSelf, Axes, BorderType, Borders, BoxSizing, Color, Dimension,
    Display, Edges, FlexDirection, FlexWrap, GridAutoFlow, GridLine, JustifyContent, JustifyItems,
    JustifySelf, LayoutSize, Length, LengthAuto, Modifier, Overflow, Position, TuiSize, TuiSpacing,
    ZIndex,
};

macro_rules! layout_style_builders {
    ($(($field:ident, $type:ty, $description:literal)),+ $(,)?) => {
        $(
            #[doc = concat!("Sets ", $description, ".")]
            ///
            /// # Arguments
            ///
            /// * `value` — Layout property value to apply.
            ///
            /// # Returns
            ///
            /// A [`TuiStyle`] with the layout property applied.
            pub const fn $field(mut self, value: $type) -> Self {
                self.$field = Some(value);
                self
            }
        )+
    };
}

/// Reusable style values for terminal UI elements.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Optional terminal-cell image render size.
    pub image_size: Option<TuiSize>,
    /// Layout strategy used to generate the view box.
    pub display: Option<Display>,
    /// Box whose dimensions are controlled by authored sizes.
    pub box_sizing: Option<BoxSizing>,
    /// Horizontal and vertical overflow behavior.
    pub overflow: Option<Axes<Overflow>>,
    /// Preferred width and height.
    pub size: Option<LayoutSize<Dimension>>,
    /// Minimum width and height.
    pub min_size: Option<LayoutSize<Dimension>>,
    /// Maximum width and height.
    pub max_size: Option<LayoutSize<Dimension>>,
    /// Outer spacing around the layout box.
    pub margin: Option<Edges<LengthAuto>>,
    /// Horizontal and vertical gaps between children.
    pub gap: Option<Axes<Length>>,
    /// Flexbox main-axis direction.
    pub flex_direction: Option<FlexDirection>,
    /// Flexbox line-wrapping behavior.
    pub flex_wrap: Option<FlexWrap>,
    /// Preferred flexbox item basis.
    pub flex_basis: Option<Dimension>,
    /// Positive free-space growth factor.
    pub flex_grow: Option<f32>,
    /// Negative free-space shrink factor.
    pub flex_shrink: Option<f32>,
    /// Cross-axis alignment applied to children.
    pub align_items: Option<AlignItems>,
    /// Cross-axis alignment selected by this item.
    pub align_self: Option<AlignSelf>,
    /// Cross-axis distribution of lines or tracks.
    pub align_content: Option<AlignContent>,
    /// Inline-axis alignment applied to grid children.
    pub justify_items: Option<JustifyItems>,
    /// Inline-axis alignment selected by this item.
    pub justify_self: Option<JustifySelf>,
    /// Main-axis or inline-axis content distribution.
    pub justify_content: Option<JustifyContent>,
    /// Automatic grid item placement behavior.
    pub grid_auto_flow: Option<GridAutoFlow>,
    /// Grid row start and end placement.
    pub grid_row: Option<GridLine>,
    /// Grid column start and end placement.
    pub grid_column: Option<GridLine>,
    /// Positioning scheme applied to the layout box.
    pub position: Option<Position>,
    /// Physical offsets for positioned boxes.
    pub inset: Option<Edges<LengthAuto>>,
    /// Positioned stacking level.
    pub z_index: Option<ZIndex>,
}

impl Default for TuiStyle {
    /// Creates an empty terminal UI style.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with no visual or layout properties.
    fn default() -> Self {
        Self::new()
    }
}

impl TuiStyle {
    /// Creates an empty style.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with no visual or layout properties.
    pub const fn new() -> Self {
        Self {
            foreground: None,
            background: None,
            modifiers: None,
            borders: None,
            border_type: None,
            padding: None,
            image_size: None,
            display: None,
            box_sizing: None,
            overflow: None,
            size: None,
            min_size: None,
            max_size: None,
            margin: None,
            gap: None,
            flex_direction: None,
            flex_wrap: None,
            flex_basis: None,
            flex_grow: None,
            flex_shrink: None,
            align_items: None,
            align_self: None,
            align_content: None,
            justify_items: None,
            justify_self: None,
            justify_content: None,
            grid_auto_flow: None,
            grid_row: None,
            grid_column: None,
            position: None,
            inset: None,
            z_index: None,
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

    /// Sets the terminal-cell image render size.
    ///
    /// # Arguments
    ///
    /// * `size` — Width and height used when rendering image views.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] with the provided image size.
    pub const fn image_size(mut self, size: TuiSize) -> Self {
        self.image_size = Some(size);
        self
    }

    layout_style_builders!(
        (display, Display, "the layout display strategy"),
        (box_sizing, BoxSizing, "the authored-size box model"),
        (overflow, Axes<Overflow>, "horizontal and vertical overflow"),
        (size, LayoutSize<Dimension>, "the preferred width and height"),
        (
            min_size,
            LayoutSize<Dimension>,
            "the minimum width and height"
        ),
        (
            max_size,
            LayoutSize<Dimension>,
            "the maximum width and height"
        ),
        (margin, Edges<LengthAuto>, "outer spacing around the box"),
        (gap, Axes<Length>, "horizontal and vertical child gaps"),
        (
            flex_direction,
            FlexDirection,
            "the flexbox main-axis direction"
        ),
        (flex_wrap, FlexWrap, "flexbox line wrapping"),
        (flex_basis, Dimension, "the preferred flexbox item basis"),
        (flex_grow, f32, "the positive free-space growth factor"),
        (flex_shrink, f32, "the negative free-space shrink factor"),
        (align_items, AlignItems, "cross-axis child alignment"),
        (align_self, AlignSelf, "this item's cross-axis alignment"),
        (
            align_content,
            AlignContent,
            "cross-axis line or track distribution"
        ),
        (
            justify_items,
            JustifyItems,
            "inline-axis grid-child alignment"
        ),
        (
            justify_self,
            JustifySelf,
            "this grid item's inline-axis alignment"
        ),
        (
            justify_content,
            JustifyContent,
            "main-axis or inline-axis content distribution"
        ),
        (
            grid_auto_flow,
            GridAutoFlow,
            "automatic grid item placement"
        ),
        (grid_row, GridLine, "grid row start and end placement"),
        (grid_column, GridLine, "grid column start and end placement"),
        (position, Position, "the positioning scheme"),
        (inset, Edges<LengthAuto>, "physical positioned-box offsets"),
        (z_index, ZIndex, "the positioned stacking level"),
    );

    /// Returns the style values inherited by descendant views.
    ///
    /// Foreground color and text modifiers inherit across view boundaries.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing inheritable values from this style.
    pub const fn inherited_values(self) -> Self {
        Self {
            foreground: self.foreground,
            background: None,
            modifiers: self.modifiers,
            borders: None,
            border_type: None,
            padding: None,
            image_size: None,
            display: None,
            box_sizing: None,
            overflow: None,
            size: None,
            min_size: None,
            max_size: None,
            margin: None,
            gap: None,
            flex_direction: None,
            flex_wrap: None,
            flex_basis: None,
            flex_grow: None,
            flex_shrink: None,
            align_items: None,
            align_self: None,
            align_content: None,
            justify_items: None,
            justify_self: None,
            justify_content: None,
            grid_auto_flow: None,
            grid_row: None,
            grid_column: None,
            position: None,
            inset: None,
            z_index: None,
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
/// Unit tests for inherited terminal UI style behavior.
mod tests {
    use super::*;

    /// Verifies inherited styles keep text values and drop surface values.
    ///
    /// # Example Under Test
    ///
    /// A [`TuiStyle`] with foreground, background, modifiers, borders, border
    /// type, and padding is reduced to inherited values.
    ///
    /// # Assertions
    ///
    /// - The inherited style keeps foreground color and modifiers.
    /// - The inherited style drops background, borders, border type, and padding.
    #[test]
    fn inherited_values_keep_text_style_and_drop_surface_style() {
        let inherited = TuiStyle::new()
            .foreground(Color::Green)
            .background(Color::Blue)
            .modifier(Modifier::BOLD)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(TuiSpacing::uniform(1))
            .inherited_values();

        assert_eq!(inherited.foreground, Some(Color::Green));
        assert_eq!(inherited.modifiers, Some(Modifier::BOLD));
        assert_eq!(inherited.background, None);
        assert_eq!(inherited.borders, None);
        assert_eq!(inherited.border_type, None);
        assert_eq!(inherited.padding, None);
    }
}
