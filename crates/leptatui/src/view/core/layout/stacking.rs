//! Web-inspired stacking levels shared by retained box painters.

use crate::{Position, ZIndex};

/// Back-to-front paint category for one retained layout box.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum StackingLevel {
    /// Positioned content with a negative explicit level.
    Negative(i32),
    /// Static content participating in normal flow.
    NormalFlow,
    /// Positioned content with an automatic or explicit zero level.
    Positioned,
    /// Positioned content with a positive explicit level.
    Positive(i32),
}

impl StackingLevel {
    /// Classifies authored positioning and z-index into a paint category.
    ///
    /// Static boxes ignore z-index. Positioned boxes with automatic and
    /// explicit zero levels share one source-ordered category.
    ///
    /// # Arguments
    ///
    /// * `position` — Authored positioning scheme.
    /// * `z_index` — Resolved positioned stacking level.
    ///
    /// # Returns
    ///
    /// A [`StackingLevel`] ordered from the backmost to frontmost category.
    pub(crate) const fn new(position: Position, z_index: ZIndex) -> Self {
        if matches!(position, Position::Static) {
            return Self::NormalFlow;
        }
        match z_index {
            ZIndex::Integer(level) if level < 0 => Self::Negative(level),
            ZIndex::Auto | ZIndex::Integer(0) => Self::Positioned,
            ZIndex::Integer(level) => Self::Positive(level),
        }
    }
}
