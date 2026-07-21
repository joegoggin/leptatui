//! Theme-aware style declarations stored by stylesheet rules.

use crate::style::{
    BorderType, Borders, Color, LayoutDirection, Modifier, ThemeValue, ThemeVariables, TuiSize,
    TuiSpacing, TuiStyle,
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
    /// Layout direction declaration.
    direction: Option<Declaration<LayoutDirection>>,
    /// Image render size declaration.
    image_size: Option<Declaration<TuiSize>>,
}

impl StyleDeclarations {
    /// Creates an empty declaration set.
    pub const fn new() -> Self {
        Self {
            foreground: None,
            background: None,
            modifiers: None,
            borders: None,
            border_type: None,
            padding: None,
            direction: None,
            image_size: None,
        }
    }

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

    /// Overlays another declaration set and returns the updated declarations.
    pub fn merge(mut self, style: &Self) -> Self {
        self.overlay(style);
        self
    }

    /// Overlays all declarations from another declaration set.
    ///
    /// # Arguments
    ///
    /// * `style` — Declaration set to cascade over this set.
    pub(crate) fn overlay(&mut self, style: &Self) {
        self.overlay_matching_importance(style, |_| true);
    }

    /// Overlays only normal declarations from another declaration set.
    ///
    /// # Arguments
    ///
    /// * `style` — Declaration set to cascade over this set.
    pub(crate) fn overlay_normal(&mut self, style: &Self) {
        self.overlay_matching_importance(style, |important| !important);
    }

    /// Overlays only important declarations from another declaration set.
    ///
    /// # Arguments
    ///
    /// * `style` — Declaration set to cascade over this set.
    pub(crate) fn overlay_important(&mut self, style: &Self) {
        self.overlay_matching_importance(style, |important| important);
    }

    /// Resolves theme-backed declarations into concrete terminal style values.
    ///
    /// # Arguments
    ///
    /// * `theme` — Active theme variables used for color resolution.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing the resolved declarations.
    pub(crate) fn resolve(&self, theme: &ThemeVariables) -> TuiStyle {
        let mut style = TuiStyle::new();

        if let Some(color) = &self.foreground {
            style = style.foreground(color.value.resolve(theme));
        }

        if let Some(color) = &self.background {
            style = style.background(color.value.resolve(theme));
        }

        if let Some(modifiers) = &self.modifiers {
            style = style.modifier(modifiers.value);
        }

        if let Some(borders) = &self.borders {
            style = style.borders(borders.value);
        }

        if let Some(border_type) = &self.border_type {
            style = style.border_type(border_type.value);
        }

        if let Some(padding) = &self.padding {
            style = style.padding(padding.value);
        }

        if let Some(direction) = &self.direction {
            style = style.direction(direction.value);
        }

        if let Some(size) = &self.image_size {
            style = style.image_size(size.value);
        }

        style
    }

    /// Overlays declarations matching a caller-selected importance predicate.
    ///
    /// # Arguments
    ///
    /// * `style` — Declaration set to cascade over this set.
    /// * `matches` — Predicate that accepts declarations by importance.
    fn overlay_matching_importance(&mut self, style: &Self, matches: impl Fn(bool) -> bool) {
        if let Some(declaration) = &style.foreground
            && matches(declaration.important)
        {
            self.set_foreground(declaration.value.clone(), declaration.important);
        }

        if let Some(declaration) = &style.background
            && matches(declaration.important)
        {
            self.set_background(declaration.value.clone(), declaration.important);
        }

        if let Some(declaration) = &style.modifiers
            && matches(declaration.important)
        {
            self.set_modifier(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.borders
            && matches(declaration.important)
        {
            self.set_borders(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.border_type
            && matches(declaration.important)
        {
            self.set_border_type(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.padding
            && matches(declaration.important)
        {
            self.set_padding(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.direction
            && matches(declaration.important)
        {
            self.set_direction(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.image_size
            && matches(declaration.important)
        {
            self.set_image_size(declaration.value, declaration.important);
        }
    }

    /// Sets the foreground declaration.
    ///
    /// # Arguments
    ///
    /// * `color` — Literal or theme-backed foreground color.
    /// * `important` — Whether the declaration has important priority.
    fn set_foreground(&mut self, color: ThemeValue<Color>, important: bool) {
        set_declaration(&mut self.foreground, color, important);
    }

    /// Sets the background declaration.
    ///
    /// # Arguments
    ///
    /// * `color` — Literal or theme-backed background color.
    /// * `important` — Whether the declaration has important priority.
    fn set_background(&mut self, color: ThemeValue<Color>, important: bool) {
        set_declaration(&mut self.background, color, important);
    }

    /// Sets the text modifier declaration.
    ///
    /// # Arguments
    ///
    /// * `modifier` — Ratatui text modifier to apply.
    /// * `important` — Whether the declaration has important priority.
    fn set_modifier(&mut self, modifier: Modifier, important: bool) {
        set_declaration(&mut self.modifiers, modifier, important);
    }

    /// Sets the border visibility declaration.
    ///
    /// # Arguments
    ///
    /// * `borders` — Border sides to render.
    /// * `important` — Whether the declaration has important priority.
    fn set_borders(&mut self, borders: Borders, important: bool) {
        set_declaration(&mut self.borders, borders, important);
    }

    /// Sets the border type declaration.
    ///
    /// # Arguments
    ///
    /// * `border_type` — Ratatui border glyph set to render.
    /// * `important` — Whether the declaration has important priority.
    fn set_border_type(&mut self, border_type: BorderType, important: bool) {
        set_declaration(&mut self.border_type, border_type, important);
    }

    /// Sets the padding declaration.
    ///
    /// # Arguments
    ///
    /// * `padding` — Terminal-cell padding around view content.
    /// * `important` — Whether the declaration has important priority.
    fn set_padding(&mut self, padding: TuiSpacing, important: bool) {
        set_declaration(&mut self.padding, padding, important);
    }

    /// Sets the layout direction declaration.
    ///
    /// # Arguments
    ///
    /// * `direction` — Child layout direction for container views.
    /// * `important` — Whether the declaration has important priority.
    fn set_direction(&mut self, direction: LayoutDirection, important: bool) {
        set_declaration(&mut self.direction, direction, important);
    }

    /// Sets the image render size declaration.
    ///
    /// # Arguments
    ///
    /// * `size` — Terminal-cell size for image views.
    /// * `important` — Whether the declaration has important priority.
    fn set_image_size(&mut self, size: TuiSize, important: bool) {
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

impl From<TuiStyle> for StyleDeclarations {
    /// Creates style declarations from concrete terminal style values.
    ///
    /// # Arguments
    ///
    /// * `style` — Concrete terminal style to convert.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value containing the present style fields.
    fn from(style: TuiStyle) -> Self {
        let mut declarations = Self::new();

        if let Some(color) = style.foreground {
            declarations = declarations.foreground(color);
        }

        if let Some(color) = style.background {
            declarations = declarations.background(color);
        }

        if let Some(modifiers) = style.modifiers {
            declarations = declarations.modifier(modifiers);
        }

        if let Some(borders) = style.borders {
            declarations = declarations.borders(borders);
        }

        if let Some(border_type) = style.border_type {
            declarations = declarations.border_type(border_type);
        }

        if let Some(padding) = style.padding {
            declarations = declarations.padding(padding);
        }

        if let Some(direction) = style.direction {
            declarations = declarations.direction(direction);
        }

        if let Some(size) = style.image_size {
            declarations = declarations.image_size(size);
        }

        declarations
    }
}
