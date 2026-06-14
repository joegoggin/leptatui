//! Terminal viewport media queries for responsive styles.

use ratatui::layout::Rect;

/// Root terminal viewport size measured in terminal cells.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ViewportSize {
    /// Terminal viewport width in cells.
    pub width: u16,
    /// Terminal viewport height in cells.
    pub height: u16,
}

impl ViewportSize {
    /// Creates a viewport size from terminal-cell dimensions.
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

impl From<Rect> for ViewportSize {
    fn from(rect: Rect) -> Self {
        Self::new(rect.width, rect.height)
    }
}

/// A CSS-like media query matched against the root terminal viewport.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MediaQuery {
    min_width: Option<u16>,
    max_width: Option<u16>,
    min_height: Option<u16>,
    max_height: Option<u16>,
}

impl MediaQuery {
    /// Creates a media query with no constraints.
    pub const fn new() -> Self {
        Self {
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
        }
    }

    /// Creates a query requiring at least `width` terminal columns.
    pub const fn min_width(width: u16) -> Self {
        Self {
            min_width: Some(width),
            max_width: None,
            min_height: None,
            max_height: None,
        }
    }

    /// Creates a query requiring at most `width` terminal columns.
    pub const fn max_width(width: u16) -> Self {
        Self {
            min_width: None,
            max_width: Some(width),
            min_height: None,
            max_height: None,
        }
    }

    /// Creates a query requiring at least `height` terminal rows.
    pub const fn min_height(height: u16) -> Self {
        Self {
            min_width: None,
            max_width: None,
            min_height: Some(height),
            max_height: None,
        }
    }

    /// Creates a query requiring at most `height` terminal rows.
    pub const fn max_height(height: u16) -> Self {
        Self {
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: Some(height),
        }
    }

    /// Combines two queries with logical `and` semantics.
    pub fn and(mut self, other: Self) -> Self {
        self.min_width = max_bound(self.min_width, other.min_width);
        self.max_width = min_bound(self.max_width, other.max_width);
        self.min_height = max_bound(self.min_height, other.min_height);
        self.max_height = min_bound(self.max_height, other.max_height);
        self
    }

    pub(crate) fn matches(&self, viewport: ViewportSize) -> bool {
        if let Some(width) = self.min_width
            && viewport.width < width
        {
            return false;
        }

        if let Some(width) = self.max_width
            && viewport.width > width
        {
            return false;
        }

        if let Some(height) = self.min_height
            && viewport.height < height
        {
            return false;
        }

        if let Some(height) = self.max_height
            && viewport.height > height
        {
            return false;
        }

        true
    }
}

fn max_bound(left: Option<u16>, right: Option<u16>) -> Option<u16> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(bound), None) | (None, Some(bound)) => Some(bound),
        (None, None) => None,
    }
}

fn min_bound(left: Option<u16>, right: Option<u16>) -> Option<u16> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(bound), None) | (None, Some(bound)) => Some(bound),
        (None, None) => None,
    }
}
