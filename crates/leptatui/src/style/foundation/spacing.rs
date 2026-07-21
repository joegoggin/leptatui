//! Terminal-cell spacing primitives.
//!
//! This module stores side-specific padding values and converts them into
//! Ratatui [`Padding`] values for block widgets.

use ratatui::widgets::Padding;

/// Simple terminal-cell spacing values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TuiSpacing {
    /// Cells to reserve on the left.
    pub left: u16,
    /// Cells to reserve on the right.
    pub right: u16,
    /// Cells to reserve above the content.
    pub top: u16,
    /// Cells to reserve below the content.
    pub bottom: u16,
}

impl TuiSpacing {
    /// No spacing.
    pub const ZERO: Self = Self {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
    };

    /// Creates spacing with every side specified.
    ///
    /// # Arguments
    ///
    /// * `left` — Cells to reserve on the left.
    /// * `right` — Cells to reserve on the right.
    /// * `top` — Cells to reserve above the content.
    /// * `bottom` — Cells to reserve below the content.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value with each side set independently.
    pub const fn new(left: u16, right: u16, top: u16, bottom: u16) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Creates equal spacing on every side.
    ///
    /// # Arguments
    ///
    /// * `value` — Cells to reserve on each side.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value with all sides set to `value`.
    pub const fn uniform(value: u16) -> Self {
        Self::new(value, value, value, value)
    }

    /// Creates equal horizontal spacing.
    ///
    /// # Arguments
    ///
    /// * `value` — Cells to reserve on the left and right sides.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value with horizontal sides set to `value`.
    pub const fn horizontal(value: u16) -> Self {
        Self::new(value, value, 0, 0)
    }

    /// Creates equal vertical spacing.
    ///
    /// # Arguments
    ///
    /// * `value` — Cells to reserve on the top and bottom sides.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value with vertical sides set to `value`.
    pub const fn vertical(value: u16) -> Self {
        Self::new(0, 0, value, value)
    }
}

impl From<TuiSpacing> for Padding {
    /// Converts terminal-cell spacing into Ratatui padding.
    ///
    /// # Arguments
    ///
    /// * `spacing` — Leptatui spacing value to convert.
    ///
    /// # Returns
    ///
    /// A [`Padding`] value with matching side sizes.
    fn from(spacing: TuiSpacing) -> Self {
        Self::new(spacing.left, spacing.right, spacing.top, spacing.bottom)
    }
}
