//! Positioning and stacking values for layout boxes.

/// Positioning scheme applied to a layout box.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Position {
    /// Participates in normal flow and ignores authored insets.
    #[default]
    Static,
    /// Participates in normal flow and offsets painting by authored insets.
    Relative,
    /// Leaves normal flow and uses the nearest positioned containing block.
    Absolute,
    /// Leaves normal flow and uses the terminal viewport as its containing block.
    Fixed,
    /// Participates in normal flow and clamps to insets within its scroll container.
    Sticky,
}

/// Stacking level applied to a positioned layout box.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ZIndex {
    /// Uses source-order stacking without an explicit integer level.
    #[default]
    Auto,
    /// Uses the contained signed stacking level.
    Integer(i32),
}

impl From<i32> for ZIndex {
    /// Converts an integer stacking level into an explicit z-index.
    ///
    /// # Arguments
    ///
    /// * `value` — Signed stacking level.
    ///
    /// # Returns
    ///
    /// A [`ZIndex::Integer`] value containing `value`.
    fn from(value: i32) -> Self {
        Self::Integer(value)
    }
}
