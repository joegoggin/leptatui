//! Theme-aware style declarations stored by stylesheet rules.
//!
//! # Modules
//!
//! - [`builders`] — Public fluent declaration builders.
//! - [`merge`] — Importance-aware declaration composition.
//! - [`resolve`] — Theme resolution and terminal-style conversion.

mod builders;
mod merge;
mod resolve;

use crate::style::{
    AlignContent, AlignItems, AlignSelf, Axes, BorderType, Borders, BoxSizing, Color, Dimension,
    Display, Edges, FlexDirection, FlexWrap, GridAutoFlow, GridLine, JustifyContent, JustifyItems,
    JustifySelf, LayoutSize, Length, LengthAuto, Modifier, Overflow, Position, ThemeValue, TuiSize,
    TuiSpacing, ZIndex,
};

/// One style declaration value plus its cascade importance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Declaration<T> {
    /// Declaration payload value.
    value: T,
    /// Whether the declaration was marked as important.
    important: bool,
}

impl<T> Declaration<T> {
    /// Creates a non-important declaration.
    ///
    /// # Arguments
    ///
    /// * `value` — Declaration payload value.
    ///
    /// # Returns
    ///
    /// A [`Declaration`] containing the normal-priority value.
    const fn normal(value: T) -> Self {
        Self {
            value,
            important: false,
        }
    }

    /// Creates an important declaration.
    ///
    /// # Arguments
    ///
    /// * `value` — Declaration payload value.
    ///
    /// # Returns
    ///
    /// A [`Declaration`] containing the important-priority value.
    const fn important(value: T) -> Self {
        Self {
            value,
            important: true,
        }
    }
}

/// Style declarations before runtime theme variables are resolved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleDeclarations {
    /// Foreground color declaration.
    foreground: Option<Declaration<ThemeValue<Color>>>,
    /// Background color declaration.
    background: Option<Declaration<ThemeValue<Color>>>,
    /// Text modifier declaration.
    modifiers: Option<Declaration<Modifier>>,
    /// Border visibility declaration.
    borders: Option<Declaration<Borders>>,
    /// Border glyph style declaration.
    border_type: Option<Declaration<BorderType>>,
    /// Padding declaration.
    padding: Option<Declaration<TuiSpacing>>,
    /// Image render size declaration.
    image_size: Option<Declaration<TuiSize>>,
    /// Layout display declaration.
    display: Option<Declaration<Display>>,
    /// Authored-size box-model declaration.
    box_sizing: Option<Declaration<BoxSizing>>,
    /// Overflow declaration.
    overflow: Option<Declaration<Axes<Overflow>>>,
    /// Preferred size declaration.
    size: Option<Declaration<LayoutSize<Dimension>>>,
    /// Minimum size declaration.
    min_size: Option<Declaration<LayoutSize<Dimension>>>,
    /// Maximum size declaration.
    max_size: Option<Declaration<LayoutSize<Dimension>>>,
    /// Preferred width-to-height ratio declaration.
    aspect_ratio: Option<Declaration<f32>>,
    /// Outer margin declaration.
    margin: Option<Declaration<Edges<LengthAuto>>>,
    /// Child gap declaration.
    gap: Option<Declaration<Axes<Length>>>,
    /// Flex direction declaration.
    flex_direction: Option<Declaration<FlexDirection>>,
    /// Flex wrapping declaration.
    flex_wrap: Option<Declaration<FlexWrap>>,
    /// Flex basis declaration.
    flex_basis: Option<Declaration<Dimension>>,
    /// Flex growth declaration.
    flex_grow: Option<Declaration<f32>>,
    /// Flex shrink declaration.
    flex_shrink: Option<Declaration<f32>>,
    /// Child cross-axis alignment declaration.
    align_items: Option<Declaration<AlignItems>>,
    /// Item cross-axis alignment declaration.
    align_self: Option<Declaration<AlignSelf>>,
    /// Cross-axis content distribution declaration.
    align_content: Option<Declaration<AlignContent>>,
    /// Child inline-axis alignment declaration.
    justify_items: Option<Declaration<JustifyItems>>,
    /// Item inline-axis alignment declaration.
    justify_self: Option<Declaration<JustifySelf>>,
    /// Main-axis or inline-axis content distribution declaration.
    justify_content: Option<Declaration<JustifyContent>>,
    /// Grid automatic-flow declaration.
    grid_auto_flow: Option<Declaration<GridAutoFlow>>,
    /// Grid row placement declaration.
    grid_row: Option<Declaration<GridLine>>,
    /// Grid column placement declaration.
    grid_column: Option<Declaration<GridLine>>,
    /// Positioning scheme declaration.
    position: Option<Declaration<Position>>,
    /// Positioned inset declaration.
    inset: Option<Declaration<Edges<LengthAuto>>>,
    /// Positioned stacking-level declaration.
    z_index: Option<Declaration<ZIndex>>,
}

impl StyleDeclarations {
    /// Creates an empty declaration set.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with every property unset.
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
            aspect_ratio: None,
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
}
