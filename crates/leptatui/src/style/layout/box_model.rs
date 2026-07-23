//! Box generation, sizing, and overflow values.

/// Layout strategy used to generate a view box.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Display {
    /// Participates in normal block layout.
    #[default]
    Block,
    /// Lays out children with flexbox.
    Flex,
    /// Lays out children with CSS Grid.
    Grid,
    /// Generates no layout or painted box for the view subtree.
    None,
}

/// Box whose dimensions are controlled by authored size properties.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BoxSizing {
    /// Applies authored sizes to the content box.
    #[default]
    ContentBox,
    /// Applies authored sizes to the border box.
    BorderBox,
}

/// Behavior for content that exceeds a layout box on one axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Overflow {
    /// Paints overflowing content without clipping it.
    #[default]
    Visible,
    /// Clips overflowing content while retaining scroll-container semantics.
    Hidden,
    /// Clips overflowing content without creating a scroll container.
    Clip,
    /// Always enables scrolling and reserves terminal scrollbar space.
    Scroll,
    /// Enables scrolling and reserves terminal scrollbar space when needed.
    Auto,
}
