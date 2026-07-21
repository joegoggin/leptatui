//! Runtime theme variables used by stylesheet resolution.

use std::collections::BTreeMap;

use crate::style::Color;

/// Named runtime values supplied by the active application theme.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ThemeVariables {
    /// Named color variables keyed by stylesheet variable name.
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

    /// Returns a named color variable or panics when it is missing.
    ///
    /// # Arguments
    ///
    /// * `name` — Theme color variable name to resolve.
    ///
    /// # Returns
    ///
    /// A [`Color`] stored for the requested name.
    ///
    /// # Panics
    ///
    /// Panics if no color variable exists for `name`.
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
    /// Creates a literal theme value from a concrete value.
    ///
    /// # Arguments
    ///
    /// * `value` — Concrete value that does not require theme resolution.
    ///
    /// # Returns
    ///
    /// A [`ThemeValue`] containing the literal value.
    fn from(value: T) -> Self {
        Self::Literal(value)
    }
}

impl ThemeValue<Color> {
    /// Resolves a color value against active theme variables.
    ///
    /// # Arguments
    ///
    /// * `theme` — Active theme variables used for named color lookup.
    ///
    /// # Returns
    ///
    /// A [`Color`] containing the resolved literal color.
    ///
    /// # Panics
    ///
    /// Panics if the value references a missing theme color variable.
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
