//! Fluent style declaration builders.

use super::{Declaration, StyleDeclarations};
use crate::style::{
    AlignContent, AlignItems, AlignSelf, Axes, BorderType, Borders, BoxSizing, Color, Dimension,
    Display, Edges, FlexDirection, FlexWrap, GridAutoFlow, GridLine, JustifyContent, JustifyItems,
    JustifySelf, LayoutSize, Length, LengthAuto, Modifier, Overflow, Position, ThemeValue, TuiSize,
    TuiSpacing, ZIndex,
};

macro_rules! layout_declaration_builders {
    ($(($field:ident, $important:ident, $setter:ident, $type:ty, $description:literal)),+ $(,)?) => {
        $(
            #[doc = concat!("Sets the normal ", $description, " declaration.")]
            ///
            /// # Arguments
            ///
            /// * `value` — Layout property value to declare.
            ///
            /// # Returns
            ///
            /// A [`StyleDeclarations`] value with the declaration applied.
            pub const fn $field(mut self, value: $type) -> Self {
                if !matches!(
                    self.$field,
                    Some(Declaration {
                        important: true,
                        ..
                    })
                ) {
                    self.$field = Some(Declaration::normal(value));
                }

                self
            }

            #[doc = concat!("Sets the important ", $description, " declaration.")]
            ///
            /// # Arguments
            ///
            /// * `value` — Layout property value to declare.
            ///
            /// # Returns
            ///
            /// A [`StyleDeclarations`] value with the important declaration applied.
            #[doc(hidden)]
            pub const fn $important(mut self, value: $type) -> Self {
                self.$field = Some(Declaration::important(value));
                self
            }

            #[doc = concat!("Sets the ", $description, " declaration with explicit importance.")]
            ///
            /// # Arguments
            ///
            /// * `value` — Layout property value to declare.
            /// * `important` — Whether the declaration has important priority.
            pub(super) fn $setter(&mut self, value: $type, important: bool) {
                set_declaration(&mut self.$field, value, important);
            }
        )+
    };
}

impl StyleDeclarations {
    /// Sets the normal foreground color declaration.
    ///
    /// # Arguments
    ///
    /// * `color` — Literal or theme-backed foreground color.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the foreground declaration applied.
    pub fn foreground(mut self, color: impl Into<ThemeValue<Color>>) -> Self {
        self.set_foreground(color.into(), false);
        self
    }

    /// Sets the important foreground color declaration.
    ///
    /// # Arguments
    ///
    /// * `color` — Literal or theme-backed foreground color.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the important foreground declaration applied.
    #[doc(hidden)]
    pub fn foreground_important(mut self, color: impl Into<ThemeValue<Color>>) -> Self {
        self.set_foreground(color.into(), true);
        self
    }

    /// Sets the normal background color declaration.
    ///
    /// # Arguments
    ///
    /// * `color` — Literal or theme-backed background color.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the background declaration applied.
    pub fn background(mut self, color: impl Into<ThemeValue<Color>>) -> Self {
        self.set_background(color.into(), false);
        self
    }

    /// Sets the important background color declaration.
    ///
    /// # Arguments
    ///
    /// * `color` — Literal or theme-backed background color.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the important background declaration applied.
    #[doc(hidden)]
    pub fn background_important(mut self, color: impl Into<ThemeValue<Color>>) -> Self {
        self.set_background(color.into(), true);
        self
    }

    /// Sets the normal text modifier declaration.
    ///
    /// # Arguments
    ///
    /// * `modifier` — Ratatui text modifier to apply.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the modifier declaration applied.
    pub fn modifier(mut self, modifier: Modifier) -> Self {
        self.set_modifier(modifier, false);
        self
    }

    /// Sets the important text modifier declaration.
    ///
    /// # Arguments
    ///
    /// * `modifier` — Ratatui text modifier to apply.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the important modifier declaration applied.
    #[doc(hidden)]
    pub fn modifier_important(mut self, modifier: Modifier) -> Self {
        self.set_modifier(modifier, true);
        self
    }

    /// Sets the normal border visibility declaration.
    ///
    /// # Arguments
    ///
    /// * `borders` — Border sides to render.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the border declaration applied.
    pub const fn borders(mut self, borders: Borders) -> Self {
        if !matches!(
            self.borders,
            Some(Declaration {
                important: true,
                ..
            })
        ) {
            self.borders = Some(Declaration::normal(borders));
        }

        self
    }

    /// Sets the important border visibility declaration.
    ///
    /// # Arguments
    ///
    /// * `borders` — Border sides to render.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the important border declaration applied.
    #[doc(hidden)]
    pub const fn borders_important(mut self, borders: Borders) -> Self {
        self.borders = Some(Declaration::important(borders));
        self
    }

    /// Sets the normal border type declaration.
    ///
    /// # Arguments
    ///
    /// * `border_type` — Ratatui border glyph set to render.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the border type declaration applied.
    pub const fn border_type(mut self, border_type: BorderType) -> Self {
        if !matches!(
            self.border_type,
            Some(Declaration {
                important: true,
                ..
            })
        ) {
            self.border_type = Some(Declaration::normal(border_type));
        }

        self
    }

    /// Sets the important border type declaration.
    ///
    /// # Arguments
    ///
    /// * `border_type` — Ratatui border glyph set to render.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the important border type declaration applied.
    #[doc(hidden)]
    pub const fn border_type_important(mut self, border_type: BorderType) -> Self {
        self.border_type = Some(Declaration::important(border_type));
        self
    }

    /// Sets the normal padding declaration.
    ///
    /// # Arguments
    ///
    /// * `padding` — Terminal-cell padding around view content.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the padding declaration applied.
    pub const fn padding(mut self, padding: TuiSpacing) -> Self {
        if !matches!(
            self.padding,
            Some(Declaration {
                important: true,
                ..
            })
        ) {
            self.padding = Some(Declaration::normal(padding));
        }

        self
    }

    /// Sets the important padding declaration.
    ///
    /// # Arguments
    ///
    /// * `padding` — Terminal-cell padding around view content.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the important padding declaration applied.
    #[doc(hidden)]
    pub const fn padding_important(mut self, padding: TuiSpacing) -> Self {
        self.padding = Some(Declaration::important(padding));
        self
    }

    /// Sets the normal image render size declaration.
    ///
    /// # Arguments
    ///
    /// * `size` — Terminal-cell size for image views.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the image size declaration applied.
    pub const fn image_size(mut self, size: TuiSize) -> Self {
        if !matches!(
            self.image_size,
            Some(Declaration {
                important: true,
                ..
            })
        ) {
            self.image_size = Some(Declaration::normal(size));
        }

        self
    }

    /// Sets the important image render size declaration.
    ///
    /// # Arguments
    ///
    /// * `size` — Terminal-cell size for image views.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the important image size declaration applied.
    #[doc(hidden)]
    pub const fn image_size_important(mut self, size: TuiSize) -> Self {
        self.image_size = Some(Declaration::important(size));
        self
    }

    layout_declaration_builders!(
        (display, display_important, set_display, Display, "layout display"),
        (
            box_sizing,
            box_sizing_important,
            set_box_sizing,
            BoxSizing,
            "authored-size box model"
        ),
        (
            overflow,
            overflow_important,
            set_overflow,
            Axes<Overflow>,
            "overflow"
        ),
        (
            size,
            size_important,
            set_size,
            LayoutSize<Dimension>,
            "preferred size"
        ),
        (
            min_size,
            min_size_important,
            set_min_size,
            LayoutSize<Dimension>,
            "minimum size"
        ),
        (
            max_size,
            max_size_important,
            set_max_size,
            LayoutSize<Dimension>,
            "maximum size"
        ),
        (
            aspect_ratio,
            aspect_ratio_important,
            set_aspect_ratio,
            f32,
            "preferred width-to-height ratio"
        ),
        (
            margin,
            margin_important,
            set_margin,
            Edges<LengthAuto>,
            "outer margin"
        ),
        (gap, gap_important, set_gap, Axes<Length>, "child gap"),
        (
            flex_direction,
            flex_direction_important,
            set_flex_direction,
            FlexDirection,
            "flex direction"
        ),
        (
            flex_wrap,
            flex_wrap_important,
            set_flex_wrap,
            FlexWrap,
            "flex wrapping"
        ),
        (
            flex_basis,
            flex_basis_important,
            set_flex_basis,
            Dimension,
            "flex basis"
        ),
        (
            flex_grow,
            flex_grow_important,
            set_flex_grow,
            f32,
            "flex growth"
        ),
        (
            flex_shrink,
            flex_shrink_important,
            set_flex_shrink,
            f32,
            "flex shrink"
        ),
        (
            align_items,
            align_items_important,
            set_align_items,
            AlignItems,
            "child cross-axis alignment"
        ),
        (
            align_self,
            align_self_important,
            set_align_self,
            AlignSelf,
            "item cross-axis alignment"
        ),
        (
            align_content,
            align_content_important,
            set_align_content,
            AlignContent,
            "cross-axis content distribution"
        ),
        (
            justify_items,
            justify_items_important,
            set_justify_items,
            JustifyItems,
            "child inline-axis alignment"
        ),
        (
            justify_self,
            justify_self_important,
            set_justify_self,
            JustifySelf,
            "item inline-axis alignment"
        ),
        (
            justify_content,
            justify_content_important,
            set_justify_content,
            JustifyContent,
            "main-axis or inline-axis content distribution"
        ),
        (
            grid_auto_flow,
            grid_auto_flow_important,
            set_grid_auto_flow,
            GridAutoFlow,
            "grid automatic flow"
        ),
        (
            grid_row,
            grid_row_important,
            set_grid_row,
            GridLine,
            "grid row placement"
        ),
        (
            grid_column,
            grid_column_important,
            set_grid_column,
            GridLine,
            "grid column placement"
        ),
        (
            position,
            position_important,
            set_position,
            Position,
            "positioning scheme"
        ),
        (
            inset,
            inset_important,
            set_inset,
            Edges<LengthAuto>,
            "positioned inset"
        ),
        (
            z_index,
            z_index_important,
            set_z_index,
            ZIndex,
            "positioned stacking level"
        ),
    );

    /// Sets the foreground declaration.
    ///
    /// # Arguments
    ///
    /// * `color` — Literal or theme-backed foreground color.
    /// * `important` — Whether the declaration has important priority.
    pub(super) fn set_foreground(&mut self, color: ThemeValue<Color>, important: bool) {
        set_declaration(&mut self.foreground, color, important);
    }

    /// Sets the background declaration.
    ///
    /// # Arguments
    ///
    /// * `color` — Literal or theme-backed background color.
    /// * `important` — Whether the declaration has important priority.
    pub(super) fn set_background(&mut self, color: ThemeValue<Color>, important: bool) {
        set_declaration(&mut self.background, color, important);
    }

    /// Sets the text modifier declaration.
    ///
    /// # Arguments
    ///
    /// * `modifier` — Ratatui text modifier to apply.
    /// * `important` — Whether the declaration has important priority.
    pub(super) fn set_modifier(&mut self, modifier: Modifier, important: bool) {
        set_declaration(&mut self.modifiers, modifier, important);
    }

    /// Sets the border visibility declaration.
    ///
    /// # Arguments
    ///
    /// * `borders` — Border sides to render.
    /// * `important` — Whether the declaration has important priority.
    pub(super) fn set_borders(&mut self, borders: Borders, important: bool) {
        set_declaration(&mut self.borders, borders, important);
    }

    /// Sets the border type declaration.
    ///
    /// # Arguments
    ///
    /// * `border_type` — Ratatui border glyph set to render.
    /// * `important` — Whether the declaration has important priority.
    pub(super) fn set_border_type(&mut self, border_type: BorderType, important: bool) {
        set_declaration(&mut self.border_type, border_type, important);
    }

    /// Sets the padding declaration.
    ///
    /// # Arguments
    ///
    /// * `padding` — Terminal-cell padding around view content.
    /// * `important` — Whether the declaration has important priority.
    pub(super) fn set_padding(&mut self, padding: TuiSpacing, important: bool) {
        set_declaration(&mut self.padding, padding, important);
    }

    /// Sets the image render size declaration.
    ///
    /// # Arguments
    ///
    /// * `size` — Terminal-cell size for image views.
    /// * `important` — Whether the declaration has important priority.
    pub(super) fn set_image_size(&mut self, size: TuiSize, important: bool) {
        set_declaration(&mut self.image_size, size, important);
    }
}

/// Stores a declaration while preserving existing important values.
///
/// # Arguments
///
/// * `slot` — Declaration storage slot to update.
/// * `value` — New declaration value.
/// * `important` — Whether the new declaration has important priority.
fn set_declaration<T>(slot: &mut Option<Declaration<T>>, value: T, important: bool) {
    match slot {
        Some(existing) if existing.important && !important => {}
        _ if important => *slot = Some(Declaration::important(value)),
        _ => *slot = Some(Declaration::normal(value)),
    }
}
