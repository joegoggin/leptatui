//! Track sizing, automatic-flow, and item-placement values for CSS Grid.

use super::{Fraction, Length};

/// Minimum bound accepted by a CSS Grid `minmax()` track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridMinTrackSize {
    /// Uses a definite terminal or relative length.
    Length(Length),
    /// Lets layout derive the minimum from the track contents.
    Auto,
    /// Uses the smallest contribution allowed by the track contents.
    MinContent,
    /// Uses the largest intrinsic contribution from the track contents.
    MaxContent,
}

impl From<Length> for GridMinTrackSize {
    /// Converts a definite length into a minimum track bound.
    ///
    /// # Arguments
    ///
    /// * `value` — Definite length to use as the minimum.
    ///
    /// # Returns
    ///
    /// A [`GridMinTrackSize::Length`] containing `value`.
    fn from(value: Length) -> Self {
        Self::Length(value)
    }
}

/// Maximum bound accepted by a CSS Grid `minmax()` track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridMaxTrackSize {
    /// Uses a definite terminal or relative length.
    Length(Length),
    /// Consumes a weighted share of the remaining grid space.
    Fraction(Fraction),
    /// Lets layout derive the maximum from the track contents.
    Auto,
    /// Uses the smallest contribution allowed by the track contents.
    MinContent,
    /// Uses the largest intrinsic contribution from the track contents.
    MaxContent,
}

impl From<Length> for GridMaxTrackSize {
    /// Converts a definite length into a maximum track bound.
    ///
    /// # Arguments
    ///
    /// * `value` — Definite length to use as the maximum.
    ///
    /// # Returns
    ///
    /// A [`GridMaxTrackSize::Length`] containing `value`.
    fn from(value: Length) -> Self {
        Self::Length(value)
    }
}

impl From<Fraction> for GridMaxTrackSize {
    /// Converts a fractional weight into a maximum track bound.
    ///
    /// # Arguments
    ///
    /// * `value` — Fractional weight to use as the maximum.
    ///
    /// # Returns
    ///
    /// A [`GridMaxTrackSize::Fraction`] containing `value`.
    fn from(value: Fraction) -> Self {
        Self::Fraction(value)
    }
}

/// Sizing function for one explicit or automatically created grid track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridTrackSize {
    /// Uses a definite terminal or relative length.
    Length(Length),
    /// Consumes a weighted share of the remaining grid space.
    Fraction(Fraction),
    /// Lets layout derive the track size from its contents and available space.
    Auto,
    /// Uses the smallest contribution allowed by the track contents.
    MinContent,
    /// Uses the largest intrinsic contribution from the track contents.
    MaxContent,
    /// Clamps the track between independently typed minimum and maximum bounds.
    MinMax {
        /// Minimum size allowed for the track.
        min: GridMinTrackSize,
        /// Maximum size allowed for the track.
        max: GridMaxTrackSize,
    },
}

impl GridTrackSize {
    /// Creates a track with independently typed minimum and maximum bounds.
    ///
    /// # Arguments
    ///
    /// * `min` — Minimum sizing function for the track.
    /// * `max` — Maximum sizing function for the track.
    ///
    /// # Returns
    ///
    /// A [`GridTrackSize::MinMax`] containing both bounds.
    pub const fn minmax(min: GridMinTrackSize, max: GridMaxTrackSize) -> Self {
        Self::MinMax { min, max }
    }
}

impl From<Length> for GridTrackSize {
    /// Converts a definite length into a fixed grid track.
    ///
    /// # Arguments
    ///
    /// * `value` — Definite length to use for both track bounds.
    ///
    /// # Returns
    ///
    /// A [`GridTrackSize::Length`] containing `value`.
    fn from(value: Length) -> Self {
        Self::Length(value)
    }
}

impl From<Fraction> for GridTrackSize {
    /// Converts a fractional weight into a flexible grid track.
    ///
    /// # Arguments
    ///
    /// * `value` — Fractional weight for remaining grid space.
    ///
    /// # Returns
    ///
    /// A [`GridTrackSize::Fraction`] containing `value`.
    fn from(value: Fraction) -> Self {
        Self::Fraction(value)
    }
}

/// Repetition strategy for a grid-template fragment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridRepeat {
    /// Repeats the fragment the specified number of times.
    Count(u16),
    /// Adds as many fixed-size fragment copies as fit without collapsing empty tracks.
    AutoFill,
    /// Adds as many fixed-size fragment copies as fit and collapses empty tracks.
    AutoFit,
}

impl GridRepeat {
    /// Creates a counted template repetition.
    ///
    /// # Arguments
    ///
    /// * `count` — Number of times to repeat the template fragment.
    ///
    /// # Returns
    ///
    /// A [`GridRepeat::Count`] containing `count`.
    pub const fn count(count: u16) -> Self {
        Self::Count(count)
    }
}

impl From<u16> for GridRepeat {
    /// Converts a count into a repeated template strategy.
    ///
    /// # Arguments
    ///
    /// * `value` — Number of times to repeat the template fragment.
    ///
    /// # Returns
    ///
    /// A [`GridRepeat::Count`] containing `value`.
    fn from(value: u16) -> Self {
        Self::Count(value)
    }
}

/// One component in an explicit row or column grid template.
#[derive(Clone, Debug, PartialEq)]
pub enum GridTemplateTrack {
    /// Defines one non-repeated grid track.
    Track(GridTrackSize),
    /// Repeats an owned fragment of grid tracks.
    Repeat {
        /// Repetition strategy applied to the fragment.
        repetition: GridRepeat,
        /// Track sizing functions repeated as one fragment.
        tracks: Vec<GridTrackSize>,
    },
}

impl GridTemplateTrack {
    /// Creates a repeated explicit-template fragment.
    ///
    /// # Arguments
    ///
    /// * `repetition` — Counted or automatic repetition strategy.
    /// * `tracks` — Owned sizing functions repeated as one fragment.
    ///
    /// # Returns
    ///
    /// A [`GridTemplateTrack::Repeat`] containing the strategy and fragment.
    pub fn repeat(repetition: GridRepeat, tracks: Vec<GridTrackSize>) -> Self {
        Self::Repeat { repetition, tracks }
    }
}

impl From<GridTrackSize> for GridTemplateTrack {
    /// Converts one track sizing function into a template component.
    ///
    /// # Arguments
    ///
    /// * `value` — Non-repeated sizing function to store.
    ///
    /// # Returns
    ///
    /// A [`GridTemplateTrack::Track`] containing `value`.
    fn from(value: GridTrackSize) -> Self {
        Self::Track(value)
    }
}

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
