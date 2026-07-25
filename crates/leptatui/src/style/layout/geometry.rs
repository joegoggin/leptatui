//! Engine-independent geometry values for web-inspired layout.
//!
//! The types in this module represent authored layout values without resolving
//! them against a viewport, containing block, or layout engine.

/// Definite terminal layout length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    /// Absolute length measured in terminal cells.
    Cells(f32),
    /// Length measured as a percentage of the relevant containing block.
    Percent(f32),
    /// Length measured as a percentage of the terminal viewport width.
    ViewportWidth(f32),
    /// Length measured as a percentage of the terminal viewport height.
    ViewportHeight(f32),
    /// Length measured as a percentage of the smaller terminal viewport axis.
    ViewportMin(f32),
    /// Length measured as a percentage of the larger terminal viewport axis.
    ViewportMax(f32),
}

impl Length {
    /// Creates an absolute terminal-cell length.
    ///
    /// # Arguments
    ///
    /// * `value` — Number of terminal cells.
    ///
    /// # Returns
    ///
    /// A [`Length::Cells`] value containing `value`.
    pub const fn cells(value: f32) -> Self {
        Self::Cells(value)
    }

    /// Creates a containing-block percentage length.
    ///
    /// # Arguments
    ///
    /// * `value` — Percentage value where `100.0` represents the full axis.
    ///
    /// # Returns
    ///
    /// A [`Length::Percent`] value containing `value`.
    pub const fn percent(value: f32) -> Self {
        Self::Percent(value)
    }

    /// Creates a terminal viewport-width length.
    ///
    /// # Arguments
    ///
    /// * `value` — Percentage of the terminal viewport width.
    ///
    /// # Returns
    ///
    /// A [`Length::ViewportWidth`] value containing `value`.
    pub const fn vw(value: f32) -> Self {
        Self::ViewportWidth(value)
    }

    /// Creates a terminal viewport-height length.
    ///
    /// # Arguments
    ///
    /// * `value` — Percentage of the terminal viewport height.
    ///
    /// # Returns
    ///
    /// A [`Length::ViewportHeight`] value containing `value`.
    pub const fn vh(value: f32) -> Self {
        Self::ViewportHeight(value)
    }

    /// Creates a smaller-terminal-viewport-axis length.
    ///
    /// # Arguments
    ///
    /// * `value` — Percentage of the smaller terminal viewport axis.
    ///
    /// # Returns
    ///
    /// A [`Length::ViewportMin`] value containing `value`.
    pub const fn vmin(value: f32) -> Self {
        Self::ViewportMin(value)
    }

    /// Creates a larger-terminal-viewport-axis length.
    ///
    /// # Arguments
    ///
    /// * `value` — Percentage of the larger terminal viewport axis.
    ///
    /// # Returns
    ///
    /// A [`Length::ViewportMax`] value containing `value`.
    pub const fn vmax(value: f32) -> Self {
        Self::ViewportMax(value)
    }
}

/// Automatic or definite length used by margins and positioned insets.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LengthAuto {
    /// Lets layout choose the value automatically.
    #[default]
    Auto,
    /// Uses the contained definite length.
    Length(Length),
}

impl From<Length> for LengthAuto {
    /// Converts a definite length into a non-automatic layout value.
    ///
    /// # Arguments
    ///
    /// * `value` — Definite length to wrap.
    ///
    /// # Returns
    ///
    /// A [`LengthAuto::Length`] value containing `value`.
    fn from(value: Length) -> Self {
        Self::Length(value)
    }
}

/// Preferred, minimum, maximum, or flex-basis dimension.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    /// Lets layout determine the dimension automatically.
    #[default]
    Auto,
    /// Uses the contained definite length.
    Length(Length),
    /// Currently behaves like [`Dimension::Auto`].
    ///
    /// Minimum-content sizing is not yet implemented.
    MinContent,
    /// Currently behaves like [`Dimension::Auto`].
    ///
    /// Maximum-content sizing is not yet implemented.
    MaxContent,
    /// Currently behaves like [`Dimension::Length`].
    ///
    /// Intrinsic fit-content clamping is not yet implemented.
    FitContent(Length),
}

impl From<Length> for Dimension {
    /// Converts a definite length into a dimension.
    ///
    /// # Arguments
    ///
    /// * `value` — Definite length to wrap.
    ///
    /// # Returns
    ///
    /// A [`Dimension::Length`] value containing `value`.
    fn from(value: Length) -> Self {
        Self::Length(value)
    }
}

/// Fraction of remaining space used by grid track sizing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fraction {
    /// Fractional weighting applied to the remaining track space.
    pub value: f32,
}

impl Fraction {
    /// Creates a fractional track value.
    ///
    /// # Arguments
    ///
    /// * `value` — Fractional weighting for remaining space.
    ///
    /// # Returns
    ///
    /// A [`Fraction`] containing `value`.
    pub const fn new(value: f32) -> Self {
        Self { value }
    }
}

impl From<f32> for Fraction {
    /// Converts a floating-point weight into a fraction.
    ///
    /// # Arguments
    ///
    /// * `value` — Fractional weighting for remaining space.
    ///
    /// # Returns
    ///
    /// A [`Fraction`] containing `value`.
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

/// Values associated with the horizontal and vertical axes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Axes<T> {
    /// Value for the horizontal axis.
    pub x: T,
    /// Value for the vertical axis.
    pub y: T,
}

impl<T> Axes<T> {
    /// Creates independently valued layout axes.
    ///
    /// # Arguments
    ///
    /// * `x` — Value for the horizontal axis.
    /// * `y` — Value for the vertical axis.
    ///
    /// # Returns
    ///
    /// An [`Axes`] value containing both axes.
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Copy> Axes<T> {
    /// Creates equally valued layout axes.
    ///
    /// # Arguments
    ///
    /// * `value` — Value to apply to both axes.
    ///
    /// # Returns
    ///
    /// An [`Axes`] value with equal horizontal and vertical values.
    pub const fn all(value: T) -> Self {
        Self::new(value, value)
    }
}

/// Width and height values for a layout box.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutSize<T> {
    /// Horizontal size value.
    pub width: T,
    /// Vertical size value.
    pub height: T,
}

impl<T> LayoutSize<T> {
    /// Creates independently valued layout dimensions.
    ///
    /// # Arguments
    ///
    /// * `width` — Horizontal size value.
    /// * `height` — Vertical size value.
    ///
    /// # Returns
    ///
    /// A [`LayoutSize`] containing both dimensions.
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

impl<T: Copy> LayoutSize<T> {
    /// Creates equally valued layout dimensions.
    ///
    /// # Arguments
    ///
    /// * `value` — Value to apply to both dimensions.
    ///
    /// # Returns
    ///
    /// A [`LayoutSize`] with equal width and height values.
    pub const fn all(value: T) -> Self {
        Self::new(value, value)
    }
}

/// Values for the four physical edges of a layout box.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Edges<T> {
    /// Value for the top edge.
    pub top: T,
    /// Value for the right edge.
    pub right: T,
    /// Value for the bottom edge.
    pub bottom: T,
    /// Value for the left edge.
    pub left: T,
}

impl<T> Edges<T> {
    /// Creates independently valued physical edges.
    ///
    /// # Arguments
    ///
    /// * `top` — Value for the top edge.
    /// * `right` — Value for the right edge.
    /// * `bottom` — Value for the bottom edge.
    /// * `left` — Value for the left edge.
    ///
    /// # Returns
    ///
    /// An [`Edges`] value containing all four edges.
    pub const fn new(top: T, right: T, bottom: T, left: T) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

impl<T: Copy> Edges<T> {
    /// Creates equally valued physical edges.
    ///
    /// # Arguments
    ///
    /// * `value` — Value to apply to every edge.
    ///
    /// # Returns
    ///
    /// An [`Edges`] value with four equal values.
    pub const fn all(value: T) -> Self {
        Self::new(value, value, value, value)
    }

    /// Creates symmetric horizontal and vertical edges.
    ///
    /// # Arguments
    ///
    /// * `horizontal` — Value for the left and right edges.
    /// * `vertical` — Value for the top and bottom edges.
    ///
    /// # Returns
    ///
    /// An [`Edges`] value with symmetric axis values.
    pub const fn symmetric(horizontal: T, vertical: T) -> Self {
        Self::new(vertical, horizontal, vertical, horizontal)
    }
}
