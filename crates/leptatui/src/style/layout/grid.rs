//! Automatic-flow and item-placement values for CSS Grid.

/// Direction and density used to place automatic grid items.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GridAutoFlow {
    /// Fills each row before creating another row.
    #[default]
    Row,
    /// Fills each column before creating another column.
    Column,
    /// Fills rows and backfills earlier unoccupied cells.
    RowDense,
    /// Fills columns and backfills earlier unoccupied cells.
    ColumnDense,
}

/// Placement of one grid item edge.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GridPlacement {
    /// Lets grid auto-placement choose the edge.
    #[default]
    Auto,
    /// Places the edge on the signed explicit or implicit grid line.
    Line(i16),
    /// Spans the provided number of tracks from the opposite edge.
    Span(u16),
}

impl GridPlacement {
    /// Creates placement on a signed grid line.
    ///
    /// # Arguments
    ///
    /// * `line` — One-based positive or end-relative negative grid line.
    ///
    /// # Returns
    ///
    /// A [`GridPlacement::Line`] value containing `line`.
    pub const fn line(line: i16) -> Self {
        Self::Line(line)
    }

    /// Creates placement spanning a number of grid tracks.
    ///
    /// # Arguments
    ///
    /// * `tracks` — Number of grid tracks to span.
    ///
    /// # Returns
    ///
    /// A [`GridPlacement::Span`] value containing `tracks`.
    pub const fn span(tracks: u16) -> Self {
        Self::Span(tracks)
    }
}

/// Start and end placements for one grid axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GridLine {
    /// Placement of the starting grid edge.
    pub start: GridPlacement,
    /// Placement of the ending grid edge.
    pub end: GridPlacement,
}

impl GridLine {
    /// Creates start and end placements for one grid axis.
    ///
    /// # Arguments
    ///
    /// * `start` — Placement of the starting grid edge.
    /// * `end` — Placement of the ending grid edge.
    ///
    /// # Returns
    ///
    /// A [`GridLine`] containing both placements.
    pub const fn new(start: GridPlacement, end: GridPlacement) -> Self {
        Self { start, end }
    }
}
