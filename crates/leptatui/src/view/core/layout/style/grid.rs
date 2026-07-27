//! Grid-specific conversion from resolved Leptatui values to Taffy values.

use taffy::{
    geometry::Line as TaffyLine,
    style::{
        GridAutoFlow as TaffyGridAutoFlow, GridPlacement as TaffyGridPlacement,
        GridTemplateComponent as TaffyGridTemplateComponent,
        GridTemplateRepetition as TaffyGridTemplateRepetition,
        MaxTrackSizingFunction as TaffyMaxTrackSize, MinTrackSizingFunction as TaffyMinTrackSize,
        RepetitionCount as TaffyRepetitionCount, TrackSizingFunction as TaffyTrackSize,
    },
};

#[cfg(test)]
use super::to_taffy_style;
use super::{map_length, sanitize_factor};
#[cfg(test)]
use crate::Length;
use crate::{
    GridAutoFlow, GridLine, GridMaxTrackSize, GridMinTrackSize, GridPlacement, GridRepeat,
    GridTemplateTrack, GridTrackSize, ViewportSize,
};
#[cfg(test)]
use taffy::{geometry::Size as TaffySize, style::LengthPercentage};

/// Converts automatic grid placement flow.
///
/// # Arguments
///
/// * `value` — Public automatic-flow mode to convert.
///
/// # Returns
///
/// A [`TaffyGridAutoFlow`] with matching axis and density.
pub(super) fn map_grid_auto_flow(value: GridAutoFlow) -> TaffyGridAutoFlow {
    match value {
        GridAutoFlow::Row => TaffyGridAutoFlow::Row,
        GridAutoFlow::Column => TaffyGridAutoFlow::Column,
        GridAutoFlow::RowDense => TaffyGridAutoFlow::RowDense,
        GridAutoFlow::ColumnDense => TaffyGridAutoFlow::ColumnDense,
    }
}

/// Converts an explicit grid template and its repeat fragments.
///
/// # Arguments
///
/// * `values` — Public explicit-template components to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`Vec`] containing equivalent Taffy template components.
pub(super) fn map_grid_template(
    values: &[GridTemplateTrack],
    viewport: ViewportSize,
) -> Vec<TaffyGridTemplateComponent<String>> {
    values
        .iter()
        .map(|value| match value {
            GridTemplateTrack::Track(track) => {
                TaffyGridTemplateComponent::Single(map_grid_track(*track, viewport))
            }
            GridTemplateTrack::Repeat { repetition, tracks } => {
                TaffyGridTemplateComponent::Repeat(TaffyGridTemplateRepetition {
                    count: map_grid_repeat(*repetition),
                    tracks: map_grid_tracks(tracks, viewport),
                    line_names: Vec::new(),
                })
            }
        })
        .collect()
}

/// Converts automatic grid track sizing functions.
///
/// # Arguments
///
/// * `values` — Public track sizing functions to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`Vec`] containing equivalent Taffy track sizing functions.
pub(super) fn map_grid_tracks(
    values: &[GridTrackSize],
    viewport: ViewportSize,
) -> Vec<TaffyTrackSize> {
    values
        .iter()
        .map(|value| map_grid_track(*value, viewport))
        .collect()
}

/// Converts one engine-independent grid track sizing function.
///
/// # Arguments
///
/// * `value` — Public track sizing function to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`TaffyTrackSize`] with matching minimum and maximum behavior.
fn map_grid_track(value: GridTrackSize, viewport: ViewportSize) -> TaffyTrackSize {
    match value {
        GridTrackSize::Length(length) => {
            let length = map_length(length, viewport);
            TaffyTrackSize {
                min: length.into(),
                max: length.into(),
            }
        }
        GridTrackSize::Fraction(fraction) => TaffyTrackSize {
            min: TaffyMinTrackSize::auto(),
            max: TaffyMaxTrackSize::fr(sanitize_factor(fraction.value)),
        },
        GridTrackSize::Auto => TaffyTrackSize {
            min: TaffyMinTrackSize::auto(),
            max: TaffyMaxTrackSize::auto(),
        },
        GridTrackSize::MinContent => TaffyTrackSize {
            min: TaffyMinTrackSize::min_content(),
            max: TaffyMaxTrackSize::min_content(),
        },
        GridTrackSize::MaxContent => TaffyTrackSize {
            min: TaffyMinTrackSize::max_content(),
            max: TaffyMaxTrackSize::max_content(),
        },
        GridTrackSize::MinMax { min, max } => TaffyTrackSize {
            min: map_grid_min_track(min, viewport),
            max: map_grid_max_track(max, viewport),
        },
    }
}

/// Converts one typed minimum track bound.
///
/// # Arguments
///
/// * `value` — Public minimum track bound to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`TaffyMinTrackSize`] with matching sizing behavior.
fn map_grid_min_track(value: GridMinTrackSize, viewport: ViewportSize) -> TaffyMinTrackSize {
    match value {
        GridMinTrackSize::Length(length) => map_length(length, viewport).into(),
        GridMinTrackSize::Auto => TaffyMinTrackSize::auto(),
        GridMinTrackSize::MinContent => TaffyMinTrackSize::min_content(),
        GridMinTrackSize::MaxContent => TaffyMinTrackSize::max_content(),
    }
}

/// Converts one typed maximum track bound.
///
/// # Arguments
///
/// * `value` — Public maximum track bound to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`TaffyMaxTrackSize`] with matching sizing behavior.
fn map_grid_max_track(value: GridMaxTrackSize, viewport: ViewportSize) -> TaffyMaxTrackSize {
    match value {
        GridMaxTrackSize::Length(length) => map_length(length, viewport).into(),
        GridMaxTrackSize::Fraction(fraction) => {
            TaffyMaxTrackSize::fr(sanitize_factor(fraction.value))
        }
        GridMaxTrackSize::Auto => TaffyMaxTrackSize::auto(),
        GridMaxTrackSize::MinContent => TaffyMaxTrackSize::min_content(),
        GridMaxTrackSize::MaxContent => TaffyMaxTrackSize::max_content(),
    }
}

/// Converts an explicit-template repetition strategy.
///
/// # Arguments
///
/// * `value` — Public repetition strategy to convert.
///
/// # Returns
///
/// A [`TaffyRepetitionCount`] with matching counted or automatic behavior.
fn map_grid_repeat(value: GridRepeat) -> TaffyRepetitionCount {
    match value {
        GridRepeat::Count(count) => TaffyRepetitionCount::Count(count),
        GridRepeat::AutoFill => TaffyRepetitionCount::AutoFill,
        GridRepeat::AutoFit => TaffyRepetitionCount::AutoFit,
    }
}

/// Converts both placements for one grid axis.
///
/// # Arguments
///
/// * `value` — Public start and end placements to convert.
///
/// # Returns
///
/// A [`TaffyLine`] containing both converted placements.
pub(super) fn map_grid_line(value: GridLine) -> TaffyLine<TaffyGridPlacement> {
    TaffyLine {
        start: map_grid_placement(value.start),
        end: map_grid_placement(value.end),
    }
}

/// Converts one grid edge placement.
///
/// # Arguments
///
/// * `value` — Public grid placement to convert.
///
/// # Returns
///
/// A [`TaffyGridPlacement`] containing automatic, line, or span placement.
fn map_grid_placement(value: GridPlacement) -> TaffyGridPlacement {
    match value {
        GridPlacement::Auto | GridPlacement::Line(0) | GridPlacement::Span(0) => {
            TaffyGridPlacement::Auto
        }
        GridPlacement::Line(line) => TaffyGridPlacement::Line(line.into()),
        GridPlacement::Span(span) => TaffyGridPlacement::Span(span),
    }
}

#[cfg(test)]
/// Unit tests for public grid-track conversion into Taffy styles.
mod tests {
    use super::*;
    use crate::{Axes, Fraction, TuiStyle, text};

    /// Verifies scalar and typed min/max track sizing converts without exposing Taffy publicly.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// 4 cells, 25%, 50vw, 2fr, auto, min-content, max-content,
    /// minmax(2 cells, 3fr), and a non-finite fraction
    /// ```
    ///
    /// # Assertions
    ///
    /// - Fixed, percentage, viewport, fractional, automatic, and intrinsic tracks map exactly.
    /// - Every typed minimum and maximum intrinsic bound maps exactly.
    /// - Non-finite fractional weights sanitize to zero.
    #[test]
    fn grid_track_conversion_covers_every_sizing_form() {
        let viewport = ViewportSize::new(80, 24);

        assert_eq!(
            map_grid_track(GridTrackSize::from(Length::cells(4.0)), viewport),
            TaffyTrackSize {
                min: TaffyMinTrackSize::length(4.0),
                max: TaffyMaxTrackSize::length(4.0),
            }
        );
        assert_eq!(
            map_grid_track(GridTrackSize::from(Length::percent(25.0)), viewport),
            TaffyTrackSize {
                min: TaffyMinTrackSize::percent(0.25),
                max: TaffyMaxTrackSize::percent(0.25),
            }
        );
        assert_eq!(
            map_grid_track(GridTrackSize::from(Length::vw(50.0)), viewport),
            TaffyTrackSize {
                min: TaffyMinTrackSize::length(40.0),
                max: TaffyMaxTrackSize::length(40.0),
            }
        );
        assert_eq!(
            map_grid_track(GridTrackSize::from(Fraction::new(2.0)), viewport),
            TaffyTrackSize {
                min: TaffyMinTrackSize::auto(),
                max: TaffyMaxTrackSize::fr(2.0),
            }
        );
        assert_eq!(
            map_grid_track(GridTrackSize::Auto, viewport),
            TaffyTrackSize {
                min: TaffyMinTrackSize::auto(),
                max: TaffyMaxTrackSize::auto(),
            }
        );
        assert_eq!(
            map_grid_track(GridTrackSize::MinContent, viewport),
            TaffyTrackSize {
                min: TaffyMinTrackSize::min_content(),
                max: TaffyMaxTrackSize::min_content(),
            }
        );
        assert_eq!(
            map_grid_track(GridTrackSize::MaxContent, viewport),
            TaffyTrackSize {
                min: TaffyMinTrackSize::max_content(),
                max: TaffyMaxTrackSize::max_content(),
            }
        );
        assert_eq!(
            map_grid_track(
                GridTrackSize::minmax(
                    GridMinTrackSize::Length(Length::cells(2.0)),
                    GridMaxTrackSize::Fraction(Fraction::new(3.0)),
                ),
                viewport,
            ),
            TaffyTrackSize {
                min: TaffyMinTrackSize::length(2.0),
                max: TaffyMaxTrackSize::fr(3.0),
            }
        );

        let minimums = [
            (GridMinTrackSize::Auto, TaffyMinTrackSize::auto()),
            (
                GridMinTrackSize::MinContent,
                TaffyMinTrackSize::min_content(),
            ),
            (
                GridMinTrackSize::MaxContent,
                TaffyMinTrackSize::max_content(),
            ),
        ];
        for (value, expected) in minimums {
            assert_eq!(map_grid_min_track(value, viewport), expected);
        }

        let maximums = [
            (GridMaxTrackSize::Auto, TaffyMaxTrackSize::auto()),
            (
                GridMaxTrackSize::MinContent,
                TaffyMaxTrackSize::min_content(),
            ),
            (
                GridMaxTrackSize::MaxContent,
                TaffyMaxTrackSize::max_content(),
            ),
        ];
        for (value, expected) in maximums {
            assert_eq!(map_grid_max_track(value, viewport), expected);
        }

        assert_eq!(
            map_grid_track(GridTrackSize::from(Fraction::new(f32::NAN)), viewport),
            TaffyTrackSize {
                min: TaffyMinTrackSize::auto(),
                max: TaffyMaxTrackSize::fr(0.0),
            }
        );
    }

    /// Verifies counted and automatic repetitions retain their track fragments.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// repeat(2, 3 cells)
    /// repeat(auto-fill, 3 cells)
    /// repeat(auto-fit, 3 cells)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Counted, auto-fill, and auto-fit strategies map to matching Taffy counts.
    /// - Every repetition retains its fixed track fragment.
    /// - Repetitions add no line names.
    #[test]
    fn grid_template_conversion_covers_every_repeat_form() {
        let viewport = ViewportSize::new(80, 24);
        let fragment = vec![GridTrackSize::from(Length::cells(3.0))];
        let template = vec![
            GridTemplateTrack::repeat(GridRepeat::count(2), fragment.clone()),
            GridTemplateTrack::repeat(GridRepeat::AutoFill, fragment.clone()),
            GridTemplateTrack::repeat(GridRepeat::AutoFit, fragment),
        ];
        let mapped = map_grid_template(&template, viewport);

        let counts = [
            TaffyRepetitionCount::Count(2),
            TaffyRepetitionCount::AutoFill,
            TaffyRepetitionCount::AutoFit,
        ];
        for (component, expected_count) in mapped.iter().zip(counts) {
            let TaffyGridTemplateComponent::Repeat(repetition) = component else {
                panic!("repeat template should map to a Taffy repetition");
            };
            assert_eq!(repetition.count, expected_count);
            assert_eq!(
                repetition.tracks,
                vec![TaffyTrackSize {
                    min: TaffyMinTrackSize::length(3.0),
                    max: TaffyMaxTrackSize::length(3.0),
                }]
            );
            assert!(repetition.line_names.is_empty());
        }
    }

    /// Verifies resolved styles assign row, column, automatic, and gap values independently.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// rows: min-content; columns: max-content
    /// auto rows: 2 cells; auto columns: 1fr
    /// gap: 10% 1 cell
    /// ```
    ///
    /// # Assertions
    ///
    /// - Explicit row and column templates populate their matching Taffy fields.
    /// - Automatic row and column tracks populate their matching Taffy fields.
    /// - Shared horizontal and vertical gaps retain percentage and cell units.
    #[test]
    fn taffy_style_receives_grid_tracks_and_shared_gaps() {
        let viewport = ViewportSize::new(80, 24);
        let row_template = vec![GridTemplateTrack::from(GridTrackSize::MinContent)];
        let column_template = vec![GridTemplateTrack::from(GridTrackSize::MaxContent)];
        let auto_rows = vec![GridTrackSize::from(Length::cells(2.0))];
        let auto_columns = vec![GridTrackSize::from(Fraction::new(1.0))];
        let style = TuiStyle::new()
            .grid_template_rows(row_template)
            .grid_template_columns(column_template)
            .grid_auto_rows(auto_rows)
            .grid_auto_columns(auto_columns)
            .gap(Axes::new(Length::percent(10.0), Length::cells(1.0)));
        let view = text("grid");

        let mapped = to_taffy_style(&view, &style, viewport);

        assert_eq!(
            mapped.grid_template_rows,
            vec![TaffyGridTemplateComponent::Single(TaffyTrackSize {
                min: TaffyMinTrackSize::min_content(),
                max: TaffyMaxTrackSize::min_content(),
            })]
        );
        assert_eq!(
            mapped.grid_template_columns,
            vec![TaffyGridTemplateComponent::Single(TaffyTrackSize {
                min: TaffyMinTrackSize::max_content(),
                max: TaffyMaxTrackSize::max_content(),
            })]
        );
        assert_eq!(
            mapped.grid_auto_rows,
            vec![TaffyTrackSize {
                min: TaffyMinTrackSize::length(2.0),
                max: TaffyMaxTrackSize::length(2.0),
            }]
        );
        assert_eq!(
            mapped.grid_auto_columns,
            vec![TaffyTrackSize {
                min: TaffyMinTrackSize::auto(),
                max: TaffyMaxTrackSize::fr(1.0),
            }]
        );
        assert_eq!(
            mapped.gap,
            TaffySize {
                width: LengthPercentage::percent(0.1),
                height: LengthPercentage::length(1.0),
            }
        );
    }
}
