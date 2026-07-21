//! Theme resolution and terminal-style conversion.

use super::StyleDeclarations;
use crate::style::{ThemeVariables, TuiStyle};

impl StyleDeclarations {
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
