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
    BorderType, Borders, Color, LayoutDirection, Modifier, ThemeValue, TuiSize, TuiSpacing,
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
}
