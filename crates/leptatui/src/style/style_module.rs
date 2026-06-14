//! Reusable stylesheet variables and mixins.
//!
//! Style modules are returned by `stylesheet!` invocations that contain
//! variables or mixins without style rules, and can be imported by another
//! `stylesheet!` invocation with `@use`.

use std::collections::BTreeMap;

use super::{
    BorderType, Borders, Color, LayoutDirection, Modifier, StyleDeclarations, ThemeValue,
    TuiSpacing,
};

/// A typed value stored in a reusable stylesheet module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleValue {
    /// Foreground or background color, either literal or theme-backed.
    Color(ThemeValue<Color>),
    /// Text modifier flags.
    Modifier(Modifier),
    /// Widget border sides.
    Borders(Borders),
    /// Widget border glyph set.
    BorderType(BorderType),
    /// Internal widget padding.
    Spacing(TuiSpacing),
    /// Child layout direction.
    LayoutDirection(LayoutDirection),
}

impl StyleValue {
    /// Returns the value kind used in runtime panic messages.
    fn kind(&self) -> &'static str {
        match self {
            Self::Color(_) => "color",
            Self::Modifier(_) => "modifier",
            Self::Borders(_) => "borders",
            Self::BorderType(_) => "border_type",
            Self::Spacing(_) => "spacing",
            Self::LayoutDirection(_) => "layout_direction",
        }
    }
}

impl From<Color> for StyleValue {
    fn from(value: Color) -> Self {
        Self::Color(value.into())
    }
}

impl From<ThemeValue<Color>> for StyleValue {
    fn from(value: ThemeValue<Color>) -> Self {
        Self::Color(value)
    }
}

impl From<Modifier> for StyleValue {
    fn from(value: Modifier) -> Self {
        Self::Modifier(value)
    }
}

impl From<Borders> for StyleValue {
    fn from(value: Borders) -> Self {
        Self::Borders(value)
    }
}

impl From<BorderType> for StyleValue {
    fn from(value: BorderType) -> Self {
        Self::BorderType(value)
    }
}

impl From<TuiSpacing> for StyleValue {
    fn from(value: TuiSpacing) -> Self {
        Self::Spacing(value)
    }
}

impl From<LayoutDirection> for StyleValue {
    fn from(value: LayoutDirection) -> Self {
        Self::LayoutDirection(value)
    }
}

/// Reusable stylesheet variables and declaration mixins.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StyleModule {
    variables: BTreeMap<String, StyleValue>,
    mixins: BTreeMap<String, StyleDeclarations>,
}

impl StyleModule {
    /// Creates an empty style module.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces a named variable and returns the updated module.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    /// * `value` — Typed stylesheet value to store.
    ///
    /// # Returns
    ///
    /// A [`StyleModule`] containing the stored variable.
    pub fn variable(mut self, name: impl Into<String>, value: impl Into<StyleValue>) -> Self {
        self.push_variable(name, value);
        self
    }

    /// Adds or replaces a named variable.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    /// * `value` — Typed stylesheet value to store.
    pub fn push_variable(&mut self, name: impl Into<String>, value: impl Into<StyleValue>) {
        self.variables.insert(name.into(), value.into());
    }

    /// Adds or replaces a named mixin and returns the updated module.
    ///
    /// # Arguments
    ///
    /// * `name` — Mixin name.
    /// * `style` — Declaration set expanded when the mixin is included.
    ///
    /// # Returns
    ///
    /// A [`StyleModule`] containing the stored mixin.
    pub fn mixin(mut self, name: impl Into<String>, style: impl Into<StyleDeclarations>) -> Self {
        self.push_mixin(name, style);
        self
    }

    /// Adds or replaces a named mixin.
    ///
    /// # Arguments
    ///
    /// * `name` — Mixin name.
    /// * `style` — Declaration set expanded when the mixin is included.
    pub fn push_mixin(&mut self, name: impl Into<String>, style: impl Into<StyleDeclarations>) {
        self.mixins.insert(name.into(), style.into());
    }

    /// Returns a stored variable.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the stored [`StyleValue`] when it exists.
    pub fn get_value(&self, name: &str) -> Option<&StyleValue> {
        self.variables.get(name)
    }

    /// Returns a stored mixin.
    ///
    /// # Arguments
    ///
    /// * `name` — Mixin name.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the stored [`StyleDeclarations`] when it exists.
    pub fn get_mixin(&self, name: &str) -> Option<&StyleDeclarations> {
        self.mixins.get(name)
    }

    /// Returns a color variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`ThemeValue`] for the stored variable.
    pub fn expect_color(&self, name: &str) -> ThemeValue<Color> {
        match self.expect_value(name) {
            StyleValue::Color(value) => value.clone(),
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected color",
                value.kind()
            ),
        }
    }

    /// Returns a modifier variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`Modifier`] for the stored variable.
    pub fn expect_modifier(&self, name: &str) -> Modifier {
        match self.expect_value(name) {
            StyleValue::Modifier(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected modifier",
                value.kind()
            ),
        }
    }

    /// Returns a borders variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`Borders`] value for the stored variable.
    pub fn expect_borders(&self, name: &str) -> Borders {
        match self.expect_value(name) {
            StyleValue::Borders(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected borders",
                value.kind()
            ),
        }
    }

    /// Returns a border type variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`BorderType`] for the stored variable.
    pub fn expect_border_type(&self, name: &str) -> BorderType {
        match self.expect_value(name) {
            StyleValue::BorderType(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected border_type",
                value.kind()
            ),
        }
    }

    /// Returns a spacing variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value for the stored variable.
    pub fn expect_spacing(&self, name: &str) -> TuiSpacing {
        match self.expect_value(name) {
            StyleValue::Spacing(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected spacing",
                value.kind()
            ),
        }
    }

    /// Returns a layout direction variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`LayoutDirection`] value for the stored variable.
    pub fn expect_layout_direction(&self, name: &str) -> LayoutDirection {
        match self.expect_value(name) {
            StyleValue::LayoutDirection(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected layout_direction",
                value.kind()
            ),
        }
    }

    /// Returns a mixin or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Mixin name.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] reference for the stored mixin.
    pub fn expect_mixin(&self, name: &str) -> &StyleDeclarations {
        self.get_mixin(name)
            .unwrap_or_else(|| panic!("unknown stylesheet module mixin `{name}`"))
    }

    /// Returns a variable or panics with a stylesheet-oriented message.
    fn expect_value(&self, name: &str) -> &StyleValue {
        self.get_value(name)
            .unwrap_or_else(|| panic!("unknown stylesheet module variable `${name}`"))
    }
}
