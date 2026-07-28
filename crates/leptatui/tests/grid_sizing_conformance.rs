//! CSS Grid sizing, gap, and alignment conformance tests.
//!
//! These fixtures exercise intrinsic and flexible track sizing, typed
//! `minmax()` bounds, row and column gaps, container and item alignment,
//! nested-grid contributions, and resize recomputation through Leptatui's
//! public style API.

use std::process::Command;

use leptatui::prelude::*;
use ratatui::layout::Rect;

mod support;

use support::{
    fixed_grid_track, fixture_size, render_component, render_view, rendered_lines,
    retained_child_rects,
};

/// Creates one explicit track from a public track-sizing value.
///
/// # Arguments
///
/// * `size` — Public sizing function assigned to the track.
///
/// # Returns
///
/// A [`GridTemplateTrack`] containing `size`.
fn track(size: GridTrackSize) -> GridTemplateTrack {
    GridTemplateTrack::from(size)
}

/// Creates one fractionally sized explicit track.
///
/// # Arguments
///
/// * `fraction` — Fractional weight assigned to the track.
///
/// # Returns
///
/// A [`GridTemplateTrack`] containing the fractional weight.
fn fractional_track(fraction: f32) -> GridTemplateTrack {
    track(GridTrackSize::from(Fraction::new(fraction)))
}

/// Verifies intrinsic tracks use terminal text min-content and max-content widths.
///
/// # Example Under Test
///
/// ```text
/// 16x2 grid
/// columns: min-content, max-content, 1fr
/// labels: "AA BBB", "CC DDDD", "E"
/// ```
///
/// # Assertions
///
/// - The min-content column uses the longest three-cell word.
/// - The max-content column preserves its seven-cell unwrapped label.
/// - The fractional column consumes the remaining six cells.
/// - Painted rows preserve the wrapped and unwrapped intrinsic content.
#[test]
fn intrinsic_tracks_use_terminal_text_contributions() -> leptatui::app::Result<()> {
    let root = div((text("AA BBB"), text("CC DDDD"), text("E")))
        .with_inline_style(
            fixture_size(16.0, 2.0)
                .display(Display::Grid)
                .overflow(Axes::all(Overflow::Visible))
                .grid_template_columns(vec![
                    track(GridTrackSize::MinContent),
                    track(GridTrackSize::MaxContent),
                    fractional_track(1.0),
                ])
                .grid_template_rows(vec![fixed_grid_track(2.0)]),
        )
        .into_view();

    let terminal = render_view(root.as_view(), 16, 2)?;

    assert_eq!(
        retained_child_rects(&root),
        [
            Rect::new(0, 0, 3, 2),
            Rect::new(3, 0, 7, 2),
            Rect::new(10, 0, 6, 2),
        ]
    );
    assert_eq!(
        rendered_lines(&terminal),
        ["AA CC DDDDE     ", "BBB             "]
    );
    Ok(())
}

/// Verifies fractions consume remaining cells after fixed tracks and gaps.
///
/// # Example Under Test
///
/// ```text
/// 14x3 grid
/// columns: 2 cells, 1fr, 2fr
/// rows: 1 cell, 1fr
/// gap: 1 cell on both axes
/// ```
///
/// # Assertions
///
/// - Two column gaps and one row gap reserve cells before fraction distribution.
/// - The fractional columns receive three and seven rounded terminal cells.
/// - The second row begins after the reserved row gap.
/// - Painted rows leave both gap axes empty.
#[test]
fn fractions_and_gaps_share_only_unreserved_space() -> leptatui::app::Result<()> {
    let root = div((text("A"), text("B"), text("C"), text("D")))
        .with_inline_style(
            fixture_size(14.0, 3.0)
                .display(Display::Grid)
                .overflow(Axes::all(Overflow::Visible))
                .grid_template_columns(vec![
                    fixed_grid_track(2.0),
                    fractional_track(1.0),
                    fractional_track(2.0),
                ])
                .grid_template_rows(vec![fixed_grid_track(1.0), fractional_track(1.0)])
                .gap(Axes::all(Length::cells(1.0))),
        )
        .into_view();

    let terminal = render_view(root.as_view(), 14, 3)?;

    assert_eq!(
        retained_child_rects(&root),
        [
            Rect::new(0, 0, 2, 1),
            Rect::new(3, 0, 3, 1),
            Rect::new(7, 0, 7, 1),
            Rect::new(0, 2, 2, 1),
        ]
    );
    assert_eq!(
        rendered_lines(&terminal),
        ["A  B   C      ", "              ", "D             "]
    );
    Ok(())
}

/// Verifies typed `minmax()` tracks and item constraints clamp grid geometry.
///
/// # Example Under Test
///
/// ```text
/// 6x3 grid
/// columns: minmax(4 cells, 1fr), 1fr
/// first item: size 5x3, max-size 3x2, centered
/// second item: min-width 2, block-end aligned
/// ```
///
/// # Assertions
///
/// - The first fractional track retains its four-cell minimum.
/// - The second fractional track receives the remaining two cells.
/// - The first item remains capped at three by two cells.
/// - The second item honors its two-cell minimum width and block-end alignment.
#[test]
fn minmax_tracks_and_item_constraints_clamp_grid_items() -> leptatui::app::Result<()> {
    let first = text("A").with_inline_style(
        TuiStyle::new()
            .size(LayoutSize::new(
                Dimension::from(Length::cells(5.0)),
                Dimension::from(Length::cells(3.0)),
            ))
            .max_size(LayoutSize::new(
                Dimension::from(Length::cells(3.0)),
                Dimension::from(Length::cells(2.0)),
            ))
            .justify_self(JustifySelf::Center)
            .align_self(AlignSelf::Center),
    );
    let second = text("B").with_inline_style(
        TuiStyle::new()
            .min_size(LayoutSize::new(
                Dimension::from(Length::cells(2.0)),
                Dimension::from(Length::cells(1.0)),
            ))
            .justify_self(JustifySelf::Start)
            .align_self(AlignSelf::End),
    );
    let root = div((first, second))
        .with_inline_style(
            fixture_size(6.0, 3.0)
                .display(Display::Grid)
                .overflow(Axes::all(Overflow::Visible))
                .grid_template_columns(vec![
                    track(GridTrackSize::minmax(
                        GridMinTrackSize::Length(Length::cells(4.0)),
                        GridMaxTrackSize::Fraction(Fraction::new(1.0)),
                    )),
                    fractional_track(1.0),
                ])
                .grid_template_rows(vec![fixed_grid_track(3.0)]),
        )
        .into_view();

    let terminal = render_view(root.as_view(), 6, 3)?;

    assert_eq!(
        retained_child_rects(&root),
        [Rect::new(1, 1, 3, 2), Rect::new(4, 2, 2, 1)]
    );
    assert_eq!(rendered_lines(&terminal), ["      ", " A    ", "    B "]);
    Ok(())
}

/// Verifies container defaults and per-item overrides align grid content.
///
/// # Example Under Test
///
/// ```text
/// 13x9 grid
/// tracks: two 2-cell columns and two 2-cell rows with 1-cell gaps
/// content: inline-end and block-center
/// items: inline-center and block-end with start/end self overrides
/// ```
///
/// # Assertions
///
/// - Content alignment positions the five-cell track area at inline-end and block-center.
/// - Default item alignment centers items inline and positions them at block-end.
/// - Self alignment overrides independently reposition the second and fourth items.
/// - Painted labels occupy the aligned terminal cells.
#[test]
fn container_and_item_alignment_position_tracks_and_children() -> leptatui::app::Result<()> {
    let fixed_item = |label| text(label).with_inline_style(fixture_size(1.0, 1.0));
    let root = div((
        fixed_item("A"),
        fixed_item("B").with_inline_style(
            fixture_size(1.0, 1.0)
                .justify_self(JustifySelf::End)
                .align_self(AlignSelf::Start),
        ),
        fixed_item("C"),
        fixed_item("D").with_inline_style(
            fixture_size(1.0, 1.0)
                .justify_self(JustifySelf::Start)
                .align_self(AlignSelf::Start),
        ),
    ))
    .with_inline_style(
        fixture_size(13.0, 9.0)
            .display(Display::Grid)
            .overflow(Axes::all(Overflow::Visible))
            .grid_template_columns(vec![fixed_grid_track(2.0), fixed_grid_track(2.0)])
            .grid_template_rows(vec![fixed_grid_track(2.0), fixed_grid_track(2.0)])
            .gap(Axes::all(Length::cells(1.0)))
            .justify_content(JustifyContent::End)
            .align_content(AlignContent::Center)
            .justify_items(JustifyItems::Center)
            .align_items(AlignItems::End),
    )
    .into_view();

    let terminal = render_view(root.as_view(), 13, 9)?;
    let rows = rendered_lines(&terminal);

    assert_eq!(
        retained_child_rects(&root),
        [
            Rect::new(9, 3, 1, 1),
            Rect::new(12, 2, 1, 1),
            Rect::new(9, 6, 1, 1),
            Rect::new(11, 5, 1, 1),
        ]
    );
    assert_eq!(rows[2], "            B");
    assert_eq!(rows[3], "         A   ");
    assert_eq!(rows[5], "           D ");
    assert_eq!(rows[6], "         C   ");
    Ok(())
}

/// Verifies nested grids contribute their intrinsic tracks to parent sizing.
///
/// # Example Under Test
///
/// ```text
/// outer 12x1 grid: max-content, 1fr
/// nested grid: max-content, max-content with a 1-cell gap
/// nested labels: "AB", "CDE"
/// ```
///
/// # Assertions
///
/// - The nested grid contributes six cells to the outer max-content track.
/// - The outer fractional sibling receives the remaining six cells.
/// - Nested children retain their two- and three-cell intrinsic rectangles.
/// - Painting preserves the nested column gap and outer track boundary.
#[test]
fn nested_grid_contributes_intrinsic_track_geometry() -> leptatui::app::Result<()> {
    let nested = div((text("AB"), text("CDE"))).with_inline_style(
        TuiStyle::new()
            .display(Display::Grid)
            .grid_template_columns(vec![
                track(GridTrackSize::MaxContent),
                track(GridTrackSize::MaxContent),
            ])
            .grid_template_rows(vec![fixed_grid_track(1.0)])
            .gap(Axes::new(Length::cells(1.0), Length::cells(0.0))),
    );
    let root = div((nested, text("X")))
        .with_inline_style(
            fixture_size(12.0, 1.0)
                .display(Display::Grid)
                .overflow(Axes::all(Overflow::Visible))
                .grid_template_columns(vec![
                    track(GridTrackSize::MaxContent),
                    fractional_track(1.0),
                ])
                .grid_template_rows(vec![fixed_grid_track(1.0)]),
        )
        .into_view();

    let terminal = render_view(root.as_view(), 12, 1)?;
    let outer_children = retained_child_rects(&root);
    let nested_root = root
        .downcast_ref::<DivView>()
        .expect("fixture root should be a DivView")
        .child_views()[0]
        .downcast_ref::<DivView>()
        .expect("first outer child should be a nested DivView");
    let nested_children = nested_root
        .child_views()
        .iter()
        .map(|child| {
            child
                .style_metadata()
                .and_then(StyleMetadata::layout_geometry)
                .expect("nested child should retain layout geometry")
                .border_box
        })
        .collect::<Vec<_>>();

    assert_eq!(
        outer_children,
        [Rect::new(0, 0, 6, 1), Rect::new(6, 0, 6, 1)]
    );
    assert_eq!(
        nested_children,
        [Rect::new(0, 0, 2, 1), Rect::new(3, 0, 3, 1)]
    );
    assert_eq!(rendered_lines(&terminal), ["AB CDEX     "]);
    Ok(())
}

/// Verifies fractional grid geometry is rebuilt after terminal resize.
///
/// # Example Under Test
///
/// ```text
/// 100%-wide grid with columns 1fr and 2fr plus a 1-cell gap
/// viewport widths: 10, then 7, then 10
/// ```
///
/// # Assertions
///
/// - The wide viewport assigns three and six cells to the items.
/// - The narrow viewport recomputes the fractions as two and four cells.
/// - Returning to the wide viewport replaces the retained narrow geometry.
#[test]
fn grid_rebuilds_fractional_geometry_after_terminal_resize() -> leptatui::app::Result<()> {
    let root = div((text("A"), text("B")))
        .with_inline_style(
            TuiStyle::new()
                .display(Display::Grid)
                .box_sizing(BoxSizing::BorderBox)
                .size(LayoutSize::new(
                    Dimension::from(Length::percent(100.0)),
                    Dimension::from(Length::cells(1.0)),
                ))
                .overflow(Axes::all(Overflow::Visible))
                .grid_template_columns(vec![fractional_track(1.0), fractional_track(2.0)])
                .grid_template_rows(vec![fixed_grid_track(1.0)])
                .gap(Axes::new(Length::cells(1.0), Length::cells(0.0))),
        )
        .into_view();

    let _wide = render_view(root.as_view(), 10, 1)?;
    assert_eq!(
        retained_child_rects(&root),
        [Rect::new(0, 0, 3, 1), Rect::new(4, 0, 6, 1)]
    );

    let _narrow = render_view(root.as_view(), 7, 1)?;
    assert_eq!(
        retained_child_rects(&root),
        [Rect::new(0, 0, 2, 1), Rect::new(3, 0, 4, 1)]
    );

    let _wide_again = render_view(root.as_view(), 10, 1)?;
    assert_eq!(
        retained_child_rects(&root),
        [Rect::new(0, 0, 3, 1), Rect::new(4, 0, 6, 1)]
    );
    Ok(())
}

/// Verifies mixed grid templates retain their expected terminal geometry.
///
/// # Example Under Test
///
/// ```text
/// 20x1 grid
/// columns: repeat(2, 2 cells), 25%, auto, 1fr
/// labels: "A", "B", "C", "DDD", "E"
/// ```
///
/// # Assertions
///
/// - The repeated fragment expands into two fixed two-cell columns.
/// - The percentage column receives five cells from the container width.
/// - The automatic column uses the intrinsic width of its three-cell label.
/// - The fractional column consumes the remaining eight cells.
/// - Painted output records every resulting track boundary.
#[test]
fn mixed_templates_expand_percentage_auto_repeat_and_fraction_tracks() -> leptatui::app::Result<()>
{
    let root = div((text("A"), text("B"), text("C"), text("DDD"), text("E")))
        .with_inline_style(
            fixture_size(20.0, 1.0)
                .display(Display::Grid)
                .overflow(Axes::all(Overflow::Visible))
                .grid_template_columns(vec![
                    GridTemplateTrack::repeat(
                        GridRepeat::count(2),
                        vec![GridTrackSize::from(Length::cells(2.0))],
                    ),
                    track(GridTrackSize::from(Length::percent(25.0))),
                    track(GridTrackSize::Auto),
                    fractional_track(1.0),
                ])
                .grid_template_rows(vec![fixed_grid_track(1.0)]),
        )
        .into_view();

    let terminal = render_view(root.as_view(), 20, 1)?;

    assert_eq!(
        retained_child_rects(&root),
        [
            Rect::new(0, 0, 2, 1),
            Rect::new(2, 0, 2, 1),
            Rect::new(4, 0, 5, 1),
            Rect::new(9, 0, 3, 1),
            Rect::new(12, 0, 8, 1),
        ]
    );
    assert_eq!(rendered_lines(&terminal), ["A B C    DDDE       "]);
    Ok(())
}

/// Verifies automatic repetitions distinguish filled and collapsed empty tracks.
///
/// # Example Under Test
///
/// ```text
/// 8x1 grid
/// columns: repeat(auto-fill | auto-fit, 2 cells)
/// two items
/// inline content alignment: end
/// ```
///
/// # Assertions
///
/// - Auto-fill creates four tracks and leaves both items at the grid start.
/// - Auto-fit collapses the two empty tracks.
/// - End alignment moves the remaining auto-fit tracks to the container end.
#[test]
fn automatic_repetitions_fill_and_collapse_empty_tracks() -> leptatui::app::Result<()> {
    let render = |repetition| -> leptatui::app::Result<_> {
        let root = div((text("A"), text("B")))
            .with_inline_style(
                fixture_size(8.0, 1.0)
                    .display(Display::Grid)
                    .grid_template_columns(vec![GridTemplateTrack::repeat(
                        repetition,
                        vec![GridTrackSize::from(Length::cells(2.0))],
                    )])
                    .grid_template_rows(vec![fixed_grid_track(1.0)])
                    .justify_content(JustifyContent::End),
            )
            .into_view();

        let terminal = render_view(root.as_view(), 8, 1)?;
        Ok((retained_child_rects(&root), rendered_lines(&terminal)))
    };

    let (fill_rects, fill_rows) = render(GridRepeat::AutoFill)?;
    assert_eq!(fill_rects, [Rect::new(0, 0, 2, 1), Rect::new(2, 0, 2, 1)]);
    assert_eq!(fill_rows, ["A B     "]);

    let (fit_rects, fit_rows) = render(GridRepeat::AutoFit)?;
    assert_eq!(fit_rects, [Rect::new(4, 0, 2, 1), Rect::new(6, 0, 2, 1)]);
    assert_eq!(fit_rows, ["    A B "]);
    Ok(())
}

/// Verifies fractional grid tracks round cumulatively to terminal cells.
///
/// # Example Under Test
///
/// ```text
/// 10x1 grid
/// columns: 1fr, 1fr, 1fr
/// labels: "A", "B", "C"
/// ```
///
/// # Assertions
///
/// - Three equal fractions round to widths of three, four, and three cells.
/// - Each item begins where the preceding rounded rectangle ends.
/// - The final item ends exactly at the ten-cell container edge.
/// - Painted output records the rounded starting column of every item.
#[test]
fn fractional_tracks_round_cumulatively_to_the_container_edge() -> leptatui::app::Result<()> {
    let root = div((text("A"), text("B"), text("C")))
        .with_inline_style(
            fixture_size(10.0, 1.0)
                .display(Display::Grid)
                .overflow(Axes::all(Overflow::Visible))
                .grid_template_columns(vec![
                    fractional_track(1.0),
                    fractional_track(1.0),
                    fractional_track(1.0),
                ])
                .grid_template_rows(vec![fixed_grid_track(1.0)]),
        )
        .into_view();

    let terminal = render_view(root.as_view(), 10, 1)?;

    assert_eq!(
        retained_child_rects(&root),
        [
            Rect::new(0, 0, 3, 1),
            Rect::new(3, 0, 4, 1),
            Rect::new(7, 0, 3, 1),
        ]
    );
    assert_eq!(rendered_lines(&terminal), ["A  B   C  "]);
    Ok(())
}

/// Responsive grid dashboard fixture.
#[component]
fn ResponsiveGridFixture() -> impl IntoView {
    stylesheet! {
        .fixture-dashboard => {
            display: Display::Grid,
            grid_template_columns: vec![
                GridTemplateTrack::from(GridTrackSize::from(Fraction::new(2.0))),
                GridTemplateTrack::from(GridTrackSize::from(Fraction::new(1.0)))
            ],
            grid_template_rows: vec![
                GridTemplateTrack::from(GridTrackSize::from(Length::cells(1.0))),
                GridTemplateTrack::from(GridTrackSize::from(Length::cells(1.0)))
            ],
            gap: Axes::all(Length::cells(1.0))
        }
        .fixture-heading => {
            grid_column: GridLine::new(
                GridPlacement::line(1),
                GridPlacement::line(-1)
            )
        }

        @media (max-width: 60) {
            .fixture-dashboard => {
                grid_template_columns: vec![
                    GridTemplateTrack::from(
                        GridTrackSize::from(Fraction::new(1.0))
                    )
                ],
                grid_template_rows: vec![
                    GridTemplateTrack::from(
                        GridTrackSize::from(Length::cells(1.0))
                    ),
                    GridTemplateTrack::from(
                        GridTrackSize::from(Length::cells(1.0))
                    ),
                    GridTemplateTrack::from(
                        GridTrackSize::from(Length::cells(1.0))
                    )
                ]
            }
        }
    }

    view! {
        <Div class="fixture-dashboard">
            <Text class="fixture-heading">"Dashboard"</Text>
            <Text>"Revenue"</Text>
            <Text>"Activity"</Text>
        </Div>
    }
}

/// Returns the first terminal position containing a symbol.
///
/// # Arguments
///
/// * `rows` — Rendered terminal rows.
/// * `symbol` — Text fragment to locate.
///
/// # Returns
///
/// A `(column, row)` pair for the first match.
fn symbol_position(rows: &[String], symbol: &str) -> (usize, usize) {
    rows.iter()
        .enumerate()
        .find_map(|(row, text)| text.find(symbol).map(|column| (column, row)))
        .unwrap_or_else(|| panic!("responsive grid symbol {symbol:?} missing from {rows:?}"))
}

/// Verifies a responsive grid recomputes across viewport and media-query changes.
///
/// # Example Under Test
///
/// ```text
/// same dashboard component
/// wide viewport: 80x6
/// narrow viewport: 40x6
/// wide viewport again: 80x6
/// breakpoint: max-width 60
/// ```
///
/// # Assertions
///
/// - The heading spans the dashboard above both panels at every viewport.
/// - Wide rendering places revenue and activity on the same row.
/// - Narrow rendering places activity below revenue after the media query applies.
/// - Returning to the wide viewport restores the original panel positions.
#[test]
fn responsive_grid_recomputes_after_viewport_and_media_query_changes() -> leptatui::app::Result<()>
{
    let mut fixture = ResponsiveGridFixture::new();

    let wide = rendered_lines(&render_component(&mut fixture, 80, 6)?);
    let wide_heading = symbol_position(&wide, "Dashboard");
    let wide_revenue = symbol_position(&wide, "Revenue");
    let wide_activity = symbol_position(&wide, "Activity");

    assert!(wide_heading.1 < wide_revenue.1);
    assert_eq!(wide_revenue.1, wide_activity.1);
    assert!(wide_activity.0 > wide_revenue.0);

    let narrow = rendered_lines(&render_component(&mut fixture, 40, 6)?);
    let narrow_heading = symbol_position(&narrow, "Dashboard");
    let narrow_revenue = symbol_position(&narrow, "Revenue");
    let narrow_activity = symbol_position(&narrow, "Activity");

    assert!(narrow_heading.1 < narrow_revenue.1);
    assert!(narrow_activity.1 > narrow_revenue.1);

    let wide_again = rendered_lines(&render_component(&mut fixture, 80, 6)?);

    assert_eq!(symbol_position(&wide_again, "Dashboard"), wide_heading,);
    assert_eq!(symbol_position(&wide_again, "Revenue"), wide_revenue);
    assert_eq!(symbol_position(&wide_again, "Activity"), wide_activity);
    Ok(())
}

/// Verifies the runnable responsive grid dashboard compiles.
///
/// # Example Under Test
///
/// ```text
/// cargo check --quiet --example responsive_grid
/// ```
///
/// # Assertions
///
/// - Cargo launches successfully for the example target.
/// - The responsive grid example exits compilation with a successful status.
#[test]
fn responsive_grid_example_compiles() {
    let output = Command::new("cargo")
        .args(["check", "--quiet", "--example", "responsive_grid"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo check should run for responsive_grid");

    assert!(
        output.status.success(),
        "cargo check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
