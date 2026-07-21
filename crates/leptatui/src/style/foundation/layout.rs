//! Layout style values for responsive view rendering.

/// Direction used to lay out child views.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutDirection {
    /// Lay children out horizontally.
    Row,
    /// Lay children out vertically.
    Column,
}
