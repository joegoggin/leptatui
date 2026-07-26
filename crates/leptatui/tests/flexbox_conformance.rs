//! Flexbox public-contract conformance tests.
//!
//! These fixtures exercise flex container and item properties through the
//! public API, retain the resulting terminal rectangles, and record the
//! painted rows produced after layout rounds engine geometry to cells.

use std::process::Command;

use leptatui::prelude::*;
use ratatui::layout::Rect;

mod support;

use support::{render_component, render_view, rendered_lines};

/// One flex item used by a conformance fixture.
struct FlexItemFixture {
    /// Text painted by the item.
    label: &'static str,
    /// Public flex style applied to the item.
    style: TuiStyle,
}

/// One flex container case and its expected terminal result.
struct FlexFixture {
    /// Diagnostic name reported when the fixture fails.
    name: &'static str,
    /// Terminal width used to render the fixture.
    width: u16,
    /// Terminal height used to render the fixture.
    height: u16,
    /// Public flex style applied to the container.
    container_style: TuiStyle,
    /// Source-ordered items placed in the container.
    items: Vec<FlexItemFixture>,
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

/// Creates a fixed-size, non-shrinking flex item fixture.
///
/// # Arguments
///
/// * `label` — Text painted by the item.
/// * `width` — Item width in terminal cells.
/// * `height` — Item height in terminal cells.
///
/// # Returns
///
/// A [`FlexItemFixture`] with a definite border-box size.
fn fixed_item(label: &'static str, width: f32, height: f32) -> FlexItemFixture {
    FlexItemFixture {
        label,
        style: fixture_size(width, height).flex_shrink(0.0),
    }
}

/// Creates an item fixture with an explicitly authored flex style.
///
/// # Arguments
///
/// * `label` — Text painted by the item.
/// * `style` — Public flex style applied to the item.
///
/// # Returns
///
/// A [`FlexItemFixture`] containing the label and style.
fn styled_item(label: &'static str, style: TuiStyle) -> FlexItemFixture {
    FlexItemFixture { label, style }
}

/// Returns the public flexbox conformance fixture matrix.
///
/// # Returns
///
/// A [`Vec`] containing direction, wrapping, gap, sizing, alignment, and
/// rounding fixtures with recorded geometry and painted rows.
fn flex_fixtures() -> Vec<FlexFixture> {
    vec![
        FlexFixture {
            name: "default row",
            width: 8,
            height: 2,
            container_style: fixture_size(8.0, 2.0)
                .display(Display::Flex)
                .align_items(AlignItems::FlexStart),
            items: vec![fixed_item("A", 2.0, 1.0), fixed_item("B", 2.0, 1.0)],
            expected_rects: vec![Rect::new(0, 0, 2, 1), Rect::new(2, 0, 2, 1)],
            expected_rows: vec!["A B     ", "        "],
        },
        FlexFixture {
            name: "row reverse",
            width: 8,
            height: 2,
            container_style: fixture_size(8.0, 2.0)
                .display(Display::Flex)
                .flex_direction(FlexDirection::RowReverse)
                .align_items(AlignItems::FlexStart),
            items: vec![fixed_item("A", 2.0, 1.0), fixed_item("B", 2.0, 1.0)],
            expected_rects: vec![Rect::new(6, 0, 2, 1), Rect::new(4, 0, 2, 1)],
            expected_rows: vec!["    B A ", "        "],
        },
        FlexFixture {
            name: "column",
            width: 4,
            height: 4,
            container_style: fixture_size(4.0, 4.0)
                .display(Display::Flex)
                .flex_direction(FlexDirection::Column)
                .align_items(AlignItems::FlexStart),
            items: vec![fixed_item("A", 2.0, 1.0), fixed_item("B", 2.0, 1.0)],
            expected_rects: vec![Rect::new(0, 0, 2, 1), Rect::new(0, 1, 2, 1)],
            expected_rows: vec!["A   ", "B   ", "    ", "    "],
        },
        FlexFixture {
            name: "column reverse",
            width: 4,
            height: 4,
            container_style: fixture_size(4.0, 4.0)
                .display(Display::Flex)
                .flex_direction(FlexDirection::ColumnReverse)
                .align_items(AlignItems::FlexStart),
            items: vec![fixed_item("A", 2.0, 1.0), fixed_item("B", 2.0, 1.0)],
            expected_rects: vec![Rect::new(0, 3, 2, 1), Rect::new(0, 2, 2, 1)],
            expected_rows: vec!["    ", "    ", "B   ", "A   "],
        },
        FlexFixture {
            name: "no wrap overflow",
            width: 5,
            height: 2,
            container_style: fixture_size(5.0, 2.0)
                .display(Display::Flex)
                .flex_wrap(FlexWrap::NoWrap)
                .align_items(AlignItems::FlexStart),
            items: vec![
                fixed_item("A", 2.0, 1.0),
                fixed_item("B", 2.0, 1.0),
                fixed_item("C", 2.0, 1.0),
            ],
            expected_rects: vec![
                Rect::new(0, 0, 2, 1),
                Rect::new(2, 0, 2, 1),
                Rect::new(4, 0, 2, 1),
            ],
            expected_rows: vec!["A B C", "     "],
        },
        FlexFixture {
            name: "wrap with gaps",
            width: 5,
            height: 4,
            container_style: fixture_size(5.0, 4.0)
                .display(Display::Flex)
                .flex_wrap(FlexWrap::Wrap)
                .gap(Axes::all(Length::cells(1.0)))
                .align_items(AlignItems::FlexStart)
                .align_content(AlignContent::FlexStart),
            items: vec![
                fixed_item("A", 2.0, 1.0),
                fixed_item("B", 2.0, 1.0),
                fixed_item("C", 2.0, 1.0),
            ],
            expected_rects: vec![
                Rect::new(0, 0, 2, 1),
                Rect::new(3, 0, 2, 1),
                Rect::new(0, 2, 2, 1),
            ],
            expected_rows: vec!["A  B ", "     ", "C    ", "     "],
        },
        FlexFixture {
            name: "wrap reverse",
            width: 5,
            height: 4,
            container_style: fixture_size(5.0, 4.0)
                .display(Display::Flex)
                .flex_wrap(FlexWrap::WrapReverse)
                .gap(Axes::all(Length::cells(1.0)))
                .align_items(AlignItems::FlexStart)
                .align_content(AlignContent::FlexStart),
            items: vec![
                fixed_item("A", 2.0, 1.0),
                fixed_item("B", 2.0, 1.0),
                fixed_item("C", 2.0, 1.0),
            ],
            expected_rects: vec![
                Rect::new(0, 3, 2, 1),
                Rect::new(3, 3, 2, 1),
                Rect::new(0, 1, 2, 1),
            ],
            expected_rows: vec!["     ", "C    ", "     ", "A  B "],
        },
        FlexFixture {
            name: "basis and grow",
            width: 10,
            height: 2,
            container_style: fixture_size(10.0, 2.0)
                .display(Display::Flex)
                .align_items(AlignItems::FlexStart),
            items: vec![
                styled_item(
                    "A",
                    TuiStyle::new()
                        .flex_basis(Dimension::from(Length::cells(2.0)))
                        .flex_grow(1.0),
                ),
                styled_item(
                    "B",
                    TuiStyle::new()
                        .flex_basis(Dimension::from(Length::cells(2.0)))
                        .flex_grow(2.0),
                ),
            ],
            expected_rects: vec![Rect::new(0, 0, 4, 1), Rect::new(4, 0, 6, 1)],
            expected_rows: vec!["A   B     ", "          "],
        },
        FlexFixture {
            name: "shrink",
            width: 6,
            height: 2,
            container_style: fixture_size(6.0, 2.0)
                .display(Display::Flex)
                .align_items(AlignItems::FlexStart),
            items: vec![
                styled_item(
                    "A",
                    TuiStyle::new()
                        .flex_basis(Dimension::from(Length::cells(4.0)))
                        .flex_shrink(1.0),
                ),
                styled_item(
                    "B",
                    TuiStyle::new()
                        .flex_basis(Dimension::from(Length::cells(4.0)))
                        .flex_shrink(1.0),
                ),
            ],
            expected_rects: vec![Rect::new(0, 0, 3, 1), Rect::new(3, 0, 3, 1)],
            expected_rows: vec!["A  B  ", "      "],
        },
        FlexFixture {
            name: "intrinsic and authored basis",
            width: 8,
            height: 2,
            container_style: fixture_size(8.0, 2.0)
                .display(Display::Flex)
                .align_items(AlignItems::FlexStart),
            items: vec![
                styled_item("ABC", TuiStyle::new().flex_shrink(0.0)),
                styled_item(
                    "D",
                    TuiStyle::new()
                        .flex_basis(Dimension::from(Length::cells(3.0)))
                        .flex_shrink(0.0),
                ),
            ],
            expected_rects: vec![Rect::new(0, 0, 3, 1), Rect::new(3, 0, 3, 1)],
            expected_rows: vec!["ABCD    ", "        "],
        },
        FlexFixture {
            name: "item and main-axis alignment",
            width: 10,
            height: 5,
            container_style: fixture_size(10.0, 5.0)
                .display(Display::Flex)
                .justify_content(JustifyContent::SpaceBetween)
                .align_items(AlignItems::Center),
            items: vec![
                fixed_item("A", 2.0, 1.0),
                styled_item(
                    "B",
                    fixture_size(2.0, 1.0)
                        .flex_shrink(0.0)
                        .align_self(AlignSelf::FlexEnd),
                ),
            ],
            expected_rects: vec![Rect::new(0, 2, 2, 1), Rect::new(8, 4, 2, 1)],
            expected_rows: vec![
                "          ",
                "          ",
                "A         ",
                "          ",
                "        B ",
            ],
        },
        FlexFixture {
            name: "wrapped line alignment",
            width: 5,
            height: 5,
            container_style: fixture_size(5.0, 5.0)
                .display(Display::Flex)
                .flex_wrap(FlexWrap::Wrap)
                .gap(Axes::new(Length::cells(1.0), Length::cells(0.0)))
                .align_items(AlignItems::FlexStart)
                .align_content(AlignContent::SpaceBetween),
            items: vec![
                fixed_item("A", 2.0, 1.0),
                fixed_item("B", 2.0, 1.0),
                fixed_item("C", 2.0, 1.0),
            ],
            expected_rects: vec![
                Rect::new(0, 0, 2, 1),
                Rect::new(3, 0, 2, 1),
                Rect::new(0, 4, 2, 1),
            ],
            expected_rows: vec!["A  B ", "     ", "     ", "     ", "C    "],
        },
        FlexFixture {
            name: "cumulative rounding",
            width: 10,
            height: 1,
            container_style: fixture_size(10.0, 1.0)
                .display(Display::Flex)
                .align_items(AlignItems::FlexStart),
            items: vec![
                styled_item(
                    "A",
                    TuiStyle::new()
                        .flex_basis(Dimension::from(Length::cells(0.0)))
                        .flex_grow(1.0),
                ),
                styled_item(
                    "B",
                    TuiStyle::new()
                        .flex_basis(Dimension::from(Length::cells(0.0)))
                        .flex_grow(1.0),
                ),
                styled_item(
                    "C",
                    TuiStyle::new()
                        .flex_basis(Dimension::from(Length::cells(0.0)))
                        .flex_grow(1.0),
                ),
            ],
            expected_rects: vec![
                Rect::new(0, 0, 3, 1),
                Rect::new(3, 0, 4, 1),
                Rect::new(7, 0, 3, 1),
            ],
            expected_rows: vec!["A  B   C  "],
        },
    ]
}

/// Builds one erased flex view from a conformance fixture.
///
/// # Arguments
///
/// * `fixture` — Fixture supplying the container and item styles.
///
/// # Returns
///
/// An [`AnyView`] containing the styled flex container.
fn fixture_view(fixture: &FlexFixture) -> AnyView {
    let children = fixture
        .items
        .iter()
        .map(|item| text(item.label).with_inline_style(item.style).into_view())
        .collect::<Vec<_>>();

    div(children)
        .with_inline_style(fixture.container_style)
        .into_view()
}

/// Returns retained child rectangles from an erased flex container.
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

/// Verifies the flexbox fixture matrix records geometry and painted output.
///
/// # Example Under Test
///
/// ```text
/// default/reversed rows and columns
/// nowrap/wrap/wrap-reverse with gaps
/// basis/grow/shrink and cumulative rounding
/// main-axis, item, self, and wrapped-line alignment
/// ```
///
/// # Assertions
///
/// - Every fixture renders successfully through public view and style APIs.
/// - Every child retains the recorded source-ordered terminal rectangle.
/// - Every terminal row exactly matches the fixture's recorded painted output.
#[test]
fn flexbox_conformance_matrix_records_geometry_and_painted_output() -> Result<()> {
    for fixture in flex_fixtures() {
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

/// Responsive navigation, content, and sidebar fixture.
#[component]
fn ResponsiveFlexFixture() -> impl IntoView {
    stylesheet! {
        .fixture-nav => {
            display: Display::Flex,
            gap: Axes::new(Length::cells(1.0), Length::cells(0.0))
        }
        .fixture-workspace => {
            display: Display::Flex,
            gap: Axes::new(Length::cells(1.0), Length::cells(0.0)),
            align_items: AlignItems::FlexStart
        }
        .fixture-content => {
            flex_basis: Dimension::from(Length::cells(0.0)),
            flex_grow: 1.0
        }
        .fixture-sidebar => {
            flex_basis: Dimension::from(Length::cells(14.0)),
            flex_shrink: 0.0
        }

        @media (max-width: 60) {
            .fixture-nav => { flex_direction: FlexDirection::Column }
            .fixture-workspace => {
                flex_direction: FlexDirection::Column,
                gap: Axes::new(Length::cells(0.0), Length::cells(1.0))
            }
            .fixture-content => { flex_basis: Dimension::Auto }
            .fixture-sidebar => {
                flex_basis: Dimension::Auto,
                flex_shrink: 1.0
            }
        }
    }

    view! {
        <Div>
            <Div class="fixture-nav">
                <Text>"Docs"</Text>
                <Text>"Examples"</Text>
            </Div>
            <Div class="fixture-workspace">
                <Div class="fixture-content"><Text>"Guide content"</Text></Div>
                <Div class="fixture-sidebar"><Text>"On this page"</Text></Div>
            </Div>
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
        .unwrap_or_else(|| panic!("responsive fixture symbol {symbol:?} missing from {rows:?}"))
}

/// Verifies responsive flex rules reflow navigation, content, and sidebar.
///
/// # Example Under Test
///
/// ```text
/// wide viewport: 80x8
/// narrow viewport: 40x12
/// breakpoint: max-width 60
/// ```
///
/// # Assertions
///
/// - Wide navigation links share a row and the sidebar follows content horizontally.
/// - Narrow navigation links occupy different rows.
/// - Narrow sidebar content paints below the main content after the workspace stacks.
#[test]
fn responsive_flex_fixture_reflows_at_the_documented_breakpoint() -> Result<()> {
    let mut wide_fixture = ResponsiveFlexFixture::new();
    let wide = rendered_lines(&render_component(&mut wide_fixture, 80, 8)?);
    let wide_docs = symbol_position(&wide, "Docs");
    let wide_examples = symbol_position(&wide, "Examples");
    let wide_content = symbol_position(&wide, "Guide content");
    let wide_sidebar = symbol_position(&wide, "On this page");

    assert_eq!(wide_docs.1, wide_examples.1);
    assert_eq!(wide_content.1, wide_sidebar.1);
    assert!(wide_sidebar.0 > wide_content.0);

    let mut narrow_fixture = ResponsiveFlexFixture::new();
    let narrow = rendered_lines(&render_component(&mut narrow_fixture, 40, 12)?);
    let narrow_docs = symbol_position(&narrow, "Docs");
    let narrow_examples = symbol_position(&narrow, "Examples");
    let narrow_content = symbol_position(&narrow, "Guide content");
    let narrow_sidebar = symbol_position(&narrow, "On this page");

    assert!(narrow_examples.1 > narrow_docs.1);
    assert!(narrow_sidebar.1 > narrow_content.1);

    Ok(())
}

/// Verifies the runnable responsive flex example compiles.
///
/// # Example Under Test
///
/// ```text
/// cargo check --quiet --example responsive_flex
/// ```
///
/// # Assertions
///
/// - Cargo launches successfully for the example target.
/// - The responsive flex example exits compilation with a successful status.
#[test]
fn responsive_flex_example_compiles() {
    let output = Command::new("cargo")
        .args(["check", "--quiet", "--example", "responsive_flex"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo check should run for responsive_flex");

    assert!(
        output.status.success(),
        "cargo check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
