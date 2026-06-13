//! Theme-aware style declarations stored by stylesheet rules.

use super::{
    BorderType, Borders, Color, Modifier, ThemeValue, ThemeVariables, TuiSpacing, TuiStyle,
};

/// One style declaration value plus its cascade importance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Declaration<T> {
    value: T,
    important: bool,
}

impl<T> Declaration<T> {
    const fn normal(value: T) -> Self {
        Self {
            value,
            important: false,
        }
    }

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
    foreground: Option<Declaration<ThemeValue<Color>>>,
    background: Option<Declaration<ThemeValue<Color>>>,
    modifiers: Option<Declaration<Modifier>>,
    borders: Option<Declaration<Borders>>,
    border_type: Option<Declaration<BorderType>>,
    padding: Option<Declaration<TuiSpacing>>,
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
        }
    }

    pub fn foreground(mut self, color: impl Into<ThemeValue<Color>>) -> Self {
        self.set_foreground(color.into(), false);
        self
    }

    #[doc(hidden)]
    pub fn foreground_important(mut self, color: impl Into<ThemeValue<Color>>) -> Self {
        self.set_foreground(color.into(), true);
        self
    }

    pub fn background(mut self, color: impl Into<ThemeValue<Color>>) -> Self {
        self.set_background(color.into(), false);
        self
    }

    #[doc(hidden)]
    pub fn background_important(mut self, color: impl Into<ThemeValue<Color>>) -> Self {
        self.set_background(color.into(), true);
        self
    }

    pub fn modifier(mut self, modifier: Modifier) -> Self {
        self.set_modifier(modifier, false);
        self
    }

    #[doc(hidden)]
    pub fn modifier_important(mut self, modifier: Modifier) -> Self {
        self.set_modifier(modifier, true);
        self
    }

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

    #[doc(hidden)]
    pub const fn borders_important(mut self, borders: Borders) -> Self {
        self.borders = Some(Declaration::important(borders));
        self
    }

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

    #[doc(hidden)]
    pub const fn border_type_important(mut self, border_type: BorderType) -> Self {
        self.border_type = Some(Declaration::important(border_type));
        self
    }

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

    #[doc(hidden)]
    pub const fn padding_important(mut self, padding: TuiSpacing) -> Self {
        self.padding = Some(Declaration::important(padding));
        self
    }

    /// Overlays another declaration set and returns the updated declarations.
    pub fn merge(mut self, style: &Self) -> Self {
        self.overlay(style);
        self
    }

    pub(crate) fn overlay(&mut self, style: &Self) {
        self.overlay_matching_importance(style, |_| true);
    }

    pub(crate) fn overlay_normal(&mut self, style: &Self) {
        self.overlay_matching_importance(style, |important| !important);
    }

    pub(crate) fn overlay_important(&mut self, style: &Self) {
        self.overlay_matching_importance(style, |important| important);
    }

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

        style
    }

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
    }

    fn set_foreground(&mut self, color: ThemeValue<Color>, important: bool) {
        set_declaration(&mut self.foreground, color, important);
    }

    fn set_background(&mut self, color: ThemeValue<Color>, important: bool) {
        set_declaration(&mut self.background, color, important);
    }

    fn set_modifier(&mut self, modifier: Modifier, important: bool) {
        set_declaration(&mut self.modifiers, modifier, important);
    }

    fn set_borders(&mut self, borders: Borders, important: bool) {
        set_declaration(&mut self.borders, borders, important);
    }

    fn set_border_type(&mut self, border_type: BorderType, important: bool) {
        set_declaration(&mut self.border_type, border_type, important);
    }

    fn set_padding(&mut self, padding: TuiSpacing, important: bool) {
        set_declaration(&mut self.padding, padding, important);
    }
}

fn set_declaration<T>(slot: &mut Option<Declaration<T>>, value: T, important: bool) {
    match slot {
        Some(existing) if existing.important && !important => {}
        _ if important => *slot = Some(Declaration::important(value)),
        _ => *slot = Some(Declaration::normal(value)),
    }
}

impl From<TuiStyle> for StyleDeclarations {
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

        declarations
    }
}
