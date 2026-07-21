//! Fluent style declaration builders.

use super::{Declaration, StyleDeclarations};
use crate::style::{
    BorderType, Borders, Color, LayoutDirection, Modifier, ThemeValue, TuiSize, TuiSpacing,
};

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

    /// Sets the normal layout direction declaration.
    ///
    /// # Arguments
    ///
    /// * `direction` — Child layout direction for container views.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the layout direction declaration applied.
    pub const fn direction(mut self, direction: LayoutDirection) -> Self {
        if !matches!(
            self.direction,
            Some(Declaration {
                important: true,
                ..
            })
        ) {
            self.direction = Some(Declaration::normal(direction));
        }

        self
    }

    /// Sets the important layout direction declaration.
    ///
    /// # Arguments
    ///
    /// * `direction` — Child layout direction for container views.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value with the important layout direction declaration applied.
    #[doc(hidden)]
    pub const fn direction_important(mut self, direction: LayoutDirection) -> Self {
        self.direction = Some(Declaration::important(direction));
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

    /// Sets the layout direction declaration.
    ///
    /// # Arguments
    ///
    /// * `direction` — Child layout direction for container views.
    /// * `important` — Whether the declaration has important priority.
    pub(super) fn set_direction(&mut self, direction: LayoutDirection, important: bool) {
        set_declaration(&mut self.direction, direction, important);
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
