//! Theme-aware style declarations stored by stylesheet rules.

use super::{
    BorderType, Borders, Color, Modifier, ThemeValue, ThemeVariables, TuiSpacing, TuiStyle,
};

/// Style declarations before runtime theme variables are resolved.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StyleDeclarations {
    foreground: Option<ThemeValue<Color>>,
    background: Option<ThemeValue<Color>>,
    modifiers: Option<Modifier>,
    borders: Option<Borders>,
    border_type: Option<BorderType>,
    padding: Option<TuiSpacing>,
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
        self.foreground = Some(color.into());
        self
    }

    pub fn background(mut self, color: impl Into<ThemeValue<Color>>) -> Self {
        self.background = Some(color.into());
        self
    }

    pub fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifiers = Some(self.modifiers.unwrap_or(Modifier::empty()) | modifier);
        self
    }

    pub const fn borders(mut self, borders: Borders) -> Self {
        self.borders = Some(borders);
        self
    }

    pub const fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_type = Some(border_type);
        self
    }

    pub const fn padding(mut self, padding: TuiSpacing) -> Self {
        self.padding = Some(padding);
        self
    }

    pub(crate) fn overlay(&mut self, style: &Self) {
        self.foreground = style.foreground.clone().or_else(|| self.foreground.clone());
        self.background = style.background.clone().or_else(|| self.background.clone());
        self.modifiers = style.modifiers.or(self.modifiers);
        self.borders = style.borders.or(self.borders);
        self.border_type = style.border_type.or(self.border_type);
        self.padding = style.padding.or(self.padding);
    }

    pub(crate) fn resolve(&self, theme: &ThemeVariables) -> TuiStyle {
        let mut style = TuiStyle::new();

        if let Some(color) = &self.foreground {
            style = style.foreground(color.resolve(theme));
        }

        if let Some(color) = &self.background {
            style = style.background(color.resolve(theme));
        }

        if let Some(modifiers) = self.modifiers {
            style = style.modifier(modifiers);
        }

        if let Some(borders) = self.borders {
            style = style.borders(borders);
        }

        if let Some(border_type) = self.border_type {
            style = style.border_type(border_type);
        }

        if let Some(padding) = self.padding {
            style = style.padding(padding);
        }

        style
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
