//! Terminal-cell size primitives.
//!
//! This module stores fixed render dimensions for styleable terminal UI
//! elements that support explicit cell-based sizing.

/// Simple terminal-cell dimensions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TuiSize {
    /// Cells to reserve horizontally.
    pub width: u16,
    /// Cells to reserve vertically.
    pub height: u16,
}

impl TuiSize {
    /// Zero-sized dimensions.
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    /// Creates a size with explicit width and height.
    ///
    /// # Arguments
    ///
    /// * `width` — Cells to reserve horizontally.
    /// * `height` — Cells to reserve vertically.
    ///
    /// # Returns
    ///
    /// A [`TuiSize`] value with both dimensions set independently.
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}
