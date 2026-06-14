//! Runtime theme variables used by stylesheet resolution.

use std::collections::BTreeMap;

use super::Color;

/// Named runtime values supplied by the active application theme.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ThemeVariables {
    colors: BTreeMap<String, Color>,
}

impl ThemeVariables {
    /// Creates an empty theme variable set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces a named color variable.
    pub fn color(mut self, name: impl Into<String>, color: Color) -> Self {
        self.colors.insert(name.into(), color);
        self
    }

    /// Returns a named color variable.
    pub fn get_color(&self, name: &str) -> Option<Color> {
        self.colors.get(name).copied()
    }

    pub(crate) fn expect_color(&self, name: &str) -> Color {
        self.get_color(name)
            .unwrap_or_else(|| panic!("missing theme color variable `{name}`"))
    }
}

/// A stylesheet value that is either literal or resolved from the active theme.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThemeValue<T> {
    /// Concrete value that does not depend on the active theme.
    Literal(T),
    /// Named runtime variable looked up in [`ThemeVariables`].
    Variable(String),
}

impl<T> ThemeValue<T> {
    /// Creates a named runtime variable reference.
    pub fn variable(name: impl Into<String>) -> Self {
        Self::Variable(name.into())
    }
}

impl<T> From<T> for ThemeValue<T> {
    fn from(value: T) -> Self {
        Self::Literal(value)
    }
}

impl ThemeValue<Color> {
    pub(crate) fn resolve(&self, theme: &ThemeVariables) -> Color {
        match self {
            Self::Literal(color) => *color,
            Self::Variable(name) => theme.expect_color(name),
        }
    }
}

/// Creates a stylesheet color value that resolves from the active theme.
pub fn theme_color(name: impl Into<String>) -> ThemeValue<Color> {
    ThemeValue::variable(name)
}
