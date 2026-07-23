//! Flexbox direction and wrapping values.

/// Direction of the flexbox main axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexDirection {
    /// Places items from left to right.
    #[default]
    Row,
    /// Places items from right to left.
    RowReverse,
    /// Places items from top to bottom.
    Column,
    /// Places items from bottom to top.
    ColumnReverse,
}

/// Wrapping behavior for flexbox lines.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexWrap {
    /// Keeps every item on one flex line.
    #[default]
    NoWrap,
    /// Wraps items onto additional flex lines.
    Wrap,
    /// Wraps items with the cross-axis line order reversed.
    WrapReverse,
}
