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
    /// Participates in normal flow and clamps to authored insets while its
    /// nearest scrollport moves.
    Sticky,
}

/// Stacking level applied to a positioned layout box.
///
/// Static boxes ignore this value. An explicit integer on a positioned box
/// establishes a local stacking context, including an integer value of zero.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ZIndex {
    /// Uses the positioned automatic layer in source order.
    #[default]
    Auto,
    /// Uses the contained signed stacking level and establishes a context.
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
