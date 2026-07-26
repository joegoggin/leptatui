//! CSS Grid placement and automatic-flow conformance tests.
//!
//! These fixtures exercise explicit line placement, spanning, implicit track
//! creation, sparse and dense packing, and collision handling through the
//! public Leptatui style API. Each case records source-ordered terminal
//! geometry and the painted rows produced by the retained layout tree.

use leptatui::prelude::*;
use ratatui::layout::Rect;

mod support;

use support::{render_view, rendered_lines};

/// One grid item used by a placement fixture.
struct GridItemFixture {
    /// Text painted by the item.
    label: &'static str,
    /// Public grid style applied to the item.
    style: TuiStyle,
}

/// One grid placement case and its expected terminal result.
struct GridFixture {
    /// Diagnostic name reported when the fixture fails.
    name: &'static str,
    /// Terminal width used to render the fixture.
    width: u16,
    /// Terminal height used to render the fixture.
    height: u16,
    /// Public grid style applied to the container.
    container_style: TuiStyle,
    /// Source-ordered items placed in the container.
    items: Vec<GridItemFixture>,
    /// Source-ordered child border boxes expected after terminal rounding.
    expected_rects: Vec<Rect>,
    /// Complete terminal rows expected after painting.
    expected_rows: Vec<&'static str>,
}

/// Returns a definite border-box size for a conformance fixture.
///
/// # Arguments
///
/// * `width` — Width in terminal cells.
/// * `height` — Height in terminal cells.
///
/// # Returns
///
/// A [`TuiStyle`] containing the requested border-box size.
fn fixture_size(width: f32, height: f32) -> TuiStyle {
    TuiStyle::new()
        .box_sizing(BoxSizing::BorderBox)
        .size(LayoutSize::new(
            Dimension::from(Length::cells(width)),
            Dimension::from(Length::cells(height)),
        ))
}

/// Creates a fixed grid track measured in terminal cells.
///
/// # Arguments
///
/// * `cells` — Track size in terminal cells.
///
/// # Returns
///
/// A [`GridTemplateTrack`] containing the fixed size.
fn fixed_track(cells: f32) -> GridTemplateTrack {
    GridTemplateTrack::from(GridTrackSize::from(Length::cells(cells)))
}

/// Creates a source-ordered grid item fixture.
///
/// # Arguments
///
/// * `label` — Text painted by the item.
/// * `style` — Public grid placement style applied to the item.
///
/// # Returns
///
/// A [`GridItemFixture`] containing the label and style.
fn grid_item(label: &'static str, style: TuiStyle) -> GridItemFixture {
    GridItemFixture { label, style }
}

/// Creates a fixed-size grid container fixture style.
///
/// # Arguments
///
/// * `width` — Container width in terminal cells.
/// * `height` — Container height in terminal cells.
/// * `columns` — Fixed explicit column sizes.
/// * `rows` — Fixed explicit row sizes.
/// * `flow` — Automatic placement direction and density.
///
/// # Returns
///
/// A [`TuiStyle`] containing the grid container configuration.
fn grid_container(
    width: f32,
    height: f32,
    columns: &[f32],
    rows: &[f32],
    flow: GridAutoFlow,
) -> TuiStyle {
    fixture_size(width, height)
        .display(Display::Grid)
        .overflow(Axes::all(Overflow::Visible))
        .grid_template_columns(columns.iter().copied().map(fixed_track).collect())
        .grid_template_rows(rows.iter().copied().map(fixed_track).collect())
        .grid_auto_flow(flow)
}

/// Returns explicit placement and implicit-track fixtures.
///
/// # Returns
///
/// A [`Vec`] containing signed line, span, implicit track, collision, and
/// invalid-placement cases.
fn explicit_fixtures() -> Vec<GridFixture> {
    vec![
        GridFixture {
            name: "signed explicit lines",
            width: 6,
            height: 4,
            container_style: grid_container(
                6.0,
                4.0,
                &[2.0, 2.0, 2.0],
                &[2.0, 2.0],
                GridAutoFlow::Row,
            ),
            items: vec![
                grid_item(
                    "A",
                    TuiStyle::new()
                        .grid_row(GridLine::new(
                            GridPlacement::line(1),
                            GridPlacement::line(2),
                        ))
                        .grid_column(GridLine::new(
                            GridPlacement::line(2),
                            GridPlacement::line(3),
                        )),
                ),
                grid_item(
                    "B",
                    TuiStyle::new()
                        .grid_row(GridLine::new(
                            GridPlacement::line(-2),
                            GridPlacement::line(-1),
                        ))
                        .grid_column(GridLine::new(
                            GridPlacement::line(1),
                            GridPlacement::span(2),
                        )),
                ),
            ],
            expected_rects: vec![Rect::new(2, 0, 2, 2), Rect::new(0, 2, 4, 2)],
            expected_rows: vec!["  A   ", "      ", "B     ", "      "],
        },
        GridFixture {
            name: "forward and backward spans",
            width: 8,
            height: 3,
            container_style: grid_container(
                8.0,
                3.0,
                &[2.0, 2.0, 2.0, 2.0],
                &[1.0, 1.0, 1.0],
                GridAutoFlow::Row,
            ),
            items: vec![
                grid_item(
                    "A",
                    TuiStyle::new()
                        .grid_row(GridLine::new(
                            GridPlacement::line(1),
                            GridPlacement::span(2),
                        ))
                        .grid_column(GridLine::new(
                            GridPlacement::line(1),
                            GridPlacement::span(3),
                        )),
                ),
                grid_item(
                    "B",
                    TuiStyle::new()
                        .grid_row(GridLine::new(
                            GridPlacement::line(3),
                            GridPlacement::line(4),
                        ))
                        .grid_column(GridLine::new(
                            GridPlacement::span(2),
                            GridPlacement::line(5),
                        )),
                ),
            ],
            expected_rects: vec![Rect::new(0, 0, 6, 2), Rect::new(4, 2, 4, 1)],
            expected_rows: vec!["A       ", "        ", "    B   "],
        },
        GridFixture {
            name: "positive implicit tracks",
            width: 8,
            height: 5,
            container_style: grid_container(8.0, 5.0, &[2.0], &[1.0], GridAutoFlow::Row)
                .grid_auto_columns(vec![GridTrackSize::from(Length::cells(3.0))])
                .grid_auto_rows(vec![GridTrackSize::from(Length::cells(2.0))]),
            items: vec![grid_item(
                "A",
                TuiStyle::new()
                    .grid_row(GridLine::new(
                        GridPlacement::line(3),
                        GridPlacement::line(4),
                    ))
                    .grid_column(GridLine::new(
                        GridPlacement::line(3),
                        GridPlacement::line(4),
                    )),
            )],
            expected_rects: vec![Rect::new(5, 3, 3, 2)],
            expected_rows: vec!["        ", "        ", "        ", "     A  ", "        "],
        },
        GridFixture {
            name: "explicit collision and automatic avoidance",
            width: 4,
            height: 2,
            container_style: grid_container(4.0, 2.0, &[2.0, 2.0], &[1.0, 1.0], GridAutoFlow::Row),
            items: vec![
                grid_item(
                    "A",
                    TuiStyle::new()
                        .grid_row(GridLine::new(
                            GridPlacement::line(1),
                            GridPlacement::line(2),
                        ))
                        .grid_column(GridLine::new(
                            GridPlacement::line(1),
                            GridPlacement::line(2),
                        )),
                ),
                grid_item(
                    "B",
                    TuiStyle::new()
                        .grid_row(GridLine::new(
                            GridPlacement::line(1),
                            GridPlacement::line(2),
                        ))
                        .grid_column(GridLine::new(
                            GridPlacement::line(1),
                            GridPlacement::line(2),
                        )),
                ),
                grid_item("C", TuiStyle::new()),
            ],
            expected_rects: vec![
                Rect::new(0, 0, 2, 1),
                Rect::new(0, 0, 2, 1),
                Rect::new(2, 0, 2, 1),
            ],
            expected_rows: vec!["B C ", "    "],
        },
        GridFixture {
            name: "zero placements become automatic",
            width: 4,
            height: 1,
            container_style: grid_container(4.0, 1.0, &[2.0, 2.0], &[1.0], GridAutoFlow::Row),
            items: vec![
                grid_item(
                    "A",
                    TuiStyle::new()
                        .grid_row(GridLine::new(GridPlacement::line(0), GridPlacement::Auto)),
                ),
                grid_item(
                    "B",
                    TuiStyle::new()
                        .grid_column(GridLine::new(GridPlacement::Auto, GridPlacement::span(0))),
                ),
            ],
            expected_rects: vec![Rect::new(0, 0, 2, 1), Rect::new(2, 0, 2, 1)],
            expected_rows: vec!["A B "],
        },
    ]
}

/// Returns sparse and dense row-flow fixtures.
///
/// # Returns
///
/// A [`Vec`] containing row-major cases that differ only by packing density.
fn row_flow_fixtures() -> Vec<GridFixture> {
    [
        (
            "row sparse leaves earlier hole",
            GridAutoFlow::Row,
            Rect::new(4, 1, 2, 1),
            vec!["A     ", "B   C ", "      "],
        ),
        (
            "row dense backfills earlier hole",
            GridAutoFlow::RowDense,
            Rect::new(4, 0, 2, 1),
            vec!["A   C ", "B     ", "      "],
        ),
    ]
    .into_iter()
    .map(|(name, flow, final_rect, expected_rows)| GridFixture {
        name,
        width: 6,
        height: 3,
        container_style: grid_container(6.0, 3.0, &[2.0, 2.0, 2.0], &[1.0, 1.0, 1.0], flow),
        items: vec![
            grid_item(
                "A",
                TuiStyle::new()
                    .grid_column(GridLine::new(GridPlacement::Auto, GridPlacement::span(2))),
            ),
            grid_item(
                "B",
                TuiStyle::new()
                    .grid_column(GridLine::new(GridPlacement::Auto, GridPlacement::span(2))),
            ),
            grid_item("C", TuiStyle::new()),
        ],
        expected_rects: vec![Rect::new(0, 0, 4, 1), Rect::new(0, 1, 4, 1), final_rect],
        expected_rows,
    })
    .collect()
}

/// Returns sparse and dense column-flow fixtures.
///
/// # Returns
///
/// A [`Vec`] containing column-major cases that differ only by packing density.
fn column_flow_fixtures() -> Vec<GridFixture> {
    [
        (
            "column sparse leaves earlier hole",
            GridAutoFlow::Column,
            Rect::new(2, 2, 2, 1),
            vec!["A B   ", "      ", "  C   "],
        ),
        (
            "column dense backfills earlier hole",
            GridAutoFlow::ColumnDense,
            Rect::new(0, 2, 2, 1),
            vec!["A B   ", "      ", "C     "],
        ),
    ]
    .into_iter()
    .map(|(name, flow, final_rect, expected_rows)| GridFixture {
        name,
        width: 6,
        height: 3,
        container_style: grid_container(6.0, 3.0, &[2.0, 2.0, 2.0], &[1.0, 1.0, 1.0], flow),
        items: vec![
            grid_item(
                "A",
                TuiStyle::new()
                    .grid_row(GridLine::new(GridPlacement::Auto, GridPlacement::span(2))),
            ),
            grid_item(
                "B",
                TuiStyle::new()
                    .grid_row(GridLine::new(GridPlacement::Auto, GridPlacement::span(2))),
            ),
            grid_item("C", TuiStyle::new()),
        ],
        expected_rects: vec![Rect::new(0, 0, 2, 2), Rect::new(2, 0, 2, 2), final_rect],
        expected_rows,
    })
    .collect()
}

/// Builds one erased grid view from a conformance fixture.
///
/// # Arguments
///
/// * `fixture` — Fixture supplying the container and item styles.
///
/// # Returns
///
/// An [`AnyView`] containing the styled grid container.
fn fixture_view(fixture: &GridFixture) -> AnyView {
    let children = fixture
        .items
        .iter()
        .map(|item| {
            text(item.label)
                .with_inline_style(item.style.clone())
                .into_view()
        })
        .collect::<Vec<_>>();

    div(children)
        .with_inline_style(fixture.container_style.clone())
        .into_view()
}

/// Returns retained child rectangles from an erased grid container.
///
/// # Arguments
///
/// * `root` — Erased division view rendered by the conformance fixture.
///
/// # Returns
///
/// A [`Vec`] containing child border boxes in source order.
fn retained_child_rects(root: &AnyView) -> Vec<Rect> {
    root.downcast_ref::<DivView>()
        .expect("fixture root should be a DivView")
        .child_views()
        .iter()
        .map(|child| {
            child
                .style_metadata()
                .and_then(StyleMetadata::layout_geometry)
                .expect("fixture child should retain layout geometry")
                .border_box
        })
        .collect()
}

/// Verifies explicit lines, spans, implicit tracks, and collisions retain terminal geometry.
///
/// # Example Under Test
///
/// ```text
/// signed positive and negative line pairs
/// forward and backward spans
/// explicitly addressed positive implicit rows and columns
/// overlapping explicit items followed by an automatic item
/// zero-valued line and span placements
/// ```
///
/// # Assertions
///
/// - Every fixture renders successfully through public view and style APIs.
/// - Every child retains its expected source-ordered terminal rectangle.
/// - Painted rows reflect spanning, implicit sizing, source-order overlap, and automatic fallback.
#[test]
fn explicit_grid_placement_records_geometry_and_painted_output() -> Result<()> {
    assert_fixtures(explicit_fixtures())
}

/// Verifies row automatic flow distinguishes sparse cursor placement from dense backfill.
///
/// # Example Under Test
///
/// ```text
/// three columns
/// two source-ordered items spanning two columns
/// one final single-column item
/// row sparse and row dense modes
/// ```
///
/// # Assertions
///
/// - Both modes place the spanning items in source order on successive rows.
/// - Sparse row flow leaves the first-row hole and places the final item after the cursor.
/// - Dense row flow backfills the first-row hole with the final item.
#[test]
fn row_auto_flow_preserves_sparse_cursor_and_dense_backfill() -> Result<()> {
    assert_fixtures(row_flow_fixtures())
}

/// Verifies column automatic flow distinguishes sparse cursor placement from dense backfill.
///
/// # Example Under Test
///
/// ```text
/// three rows
/// two source-ordered items spanning two rows
/// one final single-row item
/// column sparse and column dense modes
/// ```
///
/// # Assertions
///
/// - Both modes place the spanning items in source order on successive columns.
/// - Sparse column flow leaves the first-column hole and places the final item after the cursor.
/// - Dense column flow backfills the first-column hole with the final item.
#[test]
fn column_auto_flow_preserves_sparse_cursor_and_dense_backfill() -> Result<()> {
    assert_fixtures(column_flow_fixtures())
}

/// Asserts retained geometry and painted rows for a fixture collection.
///
/// # Arguments
///
/// * `fixtures` — Grid fixtures to render and compare.
///
/// # Returns
///
/// An empty [`Result`] after every fixture matches.
///
/// # Errors
///
/// Returns [`leptatui::Error::Io`] if terminal drawing or view rendering fails.
fn assert_fixtures(fixtures: Vec<GridFixture>) -> Result<()> {
    for fixture in fixtures {
        let root = fixture_view(&fixture);
        let terminal = render_view(root.as_view(), fixture.width, fixture.height)?;

        assert_eq!(
            retained_child_rects(&root),
            fixture.expected_rects,
            "geometry fixture: {}",
            fixture.name
        );
        assert_eq!(
            rendered_lines(&terminal),
            fixture.expected_rows,
            "paint fixture: {}",
            fixture.name
        );
    }

    Ok(())
}
