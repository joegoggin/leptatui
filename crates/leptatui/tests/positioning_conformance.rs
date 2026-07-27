//! Static, relative, and absolute positioning conformance tests.
//!
//! These tests exercise normal-flow participation, positioned subtree
//! translation, containing-block selection, opposing edge resolution,
//! percentage insets, and resize recomputation through the public style API.

use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

mod support;

use support::{draw_view, fixture_size, render_view, rendered_lines, retained_child_rects};

/// Creates physical inset edges from optional terminal-cell lengths.
///
/// # Arguments
///
/// * `top` — Optional inset from the top edge.
/// * `right` — Optional inset from the right edge.
/// * `bottom` — Optional inset from the bottom edge.
/// * `left` — Optional inset from the left edge.
///
/// # Returns
///
/// An [`Edges`] value containing definite or automatic inset lengths.
fn cell_insets(
    top: Option<f32>,
    right: Option<f32>,
    bottom: Option<f32>,
    left: Option<f32>,
) -> Edges<LengthAuto> {
    Edges::new(
        top.map_or(LengthAuto::Auto, |value| Length::cells(value).into()),
        right.map_or(LengthAuto::Auto, |value| Length::cells(value).into()),
        bottom.map_or(LengthAuto::Auto, |value| Length::cells(value).into()),
        left.map_or(LengthAuto::Auto, |value| Length::cells(value).into()),
    )
}

/// Returns one view's retained border box.
///
/// # Arguments
///
/// * `view` — Erased view whose retained geometry is inspected.
///
/// # Returns
///
/// A [`Rect`] containing the retained border-box geometry.
fn retained_rect(view: &AnyView) -> Rect {
    view.style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("positioning fixture view should retain layout geometry")
        .border_box
}

/// Verifies static insets are ignored while relative offsets retain flow space.
///
/// # Example Under Test
///
/// ```text
/// 10x4 static root
/// static 2x1 "S" with top: 2 and left: 4
/// relative 2x1 "R" with top: 1 and left: 3
/// normal-flow "N"
/// ```
///
/// # Assertions
///
/// - The static box remains at the normal-flow origin.
/// - The relative box and its text subtree move three columns and one row.
/// - The following normal-flow text starts where the relative box originally
///   reserved space.
#[test]
fn static_ignores_insets_and_relative_offsets_preserve_flow_space() -> Result<()> {
    let static_box = div((text("S"),)).with_inline_style(
        fixture_size(2.0, 1.0)
            .position(Position::Static)
            .inset(cell_insets(Some(2.0), None, None, Some(4.0))),
    );
    let relative_box = div((text("R"),)).with_inline_style(
        fixture_size(2.0, 1.0)
            .position(Position::Relative)
            .inset(cell_insets(Some(1.0), None, None, Some(3.0))),
    );
    let root = div((static_box, relative_box, text("N")))
        .with_inline_style(fixture_size(10.0, 4.0).overflow(Axes::all(Overflow::Visible)))
        .into_view();

    let terminal = render_view(root.as_view(), 10, 4)?;
    let rects = retained_child_rects(&root);
    let root_div = root
        .downcast_ref::<DivView>()
        .expect("positioning fixture root should be a DivView");
    let relative_div = root_div.child_views()[1]
        .downcast_ref::<DivView>()
        .expect("relative fixture should be a DivView");

    assert_eq!(rects[0], Rect::new(0, 0, 2, 1));
    assert_eq!(rects[1], Rect::new(3, 2, 2, 1));
    assert_eq!(
        retained_rect(&relative_div.child_views()[0]),
        Rect::new(3, 2, 2, 1)
    );
    assert_eq!(rects[2].y, 2);
    assert_eq!(rendered_lines(&terminal)[2], "N  R      ");
    Ok(())
}

/// Verifies absolute boxes leave block flow and honor explicit edge insets.
///
/// # Example Under Test
///
/// ```text
/// 12x6 relative root
/// normal-flow "A"
/// absolute 2x1 "B" at top: 1 and left: 4
/// normal-flow "C"
/// ```
///
/// # Assertions
///
/// - The absolute box is positioned at column four and row one.
/// - The following normal-flow box starts immediately after the first box.
/// - The absolute box overlaps the flow row instead of reserving its own row.
#[test]
fn absolute_boxes_leave_normal_flow() -> Result<()> {
    let absolute = div((text("B"),)).with_inline_style(
        fixture_size(2.0, 1.0)
            .position(Position::Absolute)
            .inset(cell_insets(Some(1.0), None, None, Some(4.0))),
    );
    let root = div((text("A"), absolute, text("C")))
        .with_inline_style(
            fixture_size(12.0, 6.0)
                .position(Position::Relative)
                .overflow(Axes::all(Overflow::Visible)),
        )
        .into_view();

    let terminal = render_view(root.as_view(), 12, 6)?;
    let rects = retained_child_rects(&root);

    assert_eq!(rects[0].y, 0);
    assert_eq!(rects[1], Rect::new(4, 1, 2, 1));
    assert_eq!(rects[2].y, 1);
    assert_eq!(rendered_lines(&terminal)[1], "C   B       ");
    Ok(())
}

/// Verifies opposing insets use relative precedence and absolute stretching.
///
/// # Example Under Test
///
/// ```text
/// 12x6 relative root
/// relative 2x1 box: top 1, right 9, bottom 4, left 2
/// absolute auto box: top 1, right 3, bottom 2, left 2
/// ```
///
/// # Assertions
///
/// - The relative box uses the authored left and top offsets.
/// - The relative box retains its explicit size.
/// - The absolute box spans the remaining width and height between opposing
///   insets.
#[test]
fn opposing_insets_follow_positioning_rules() -> Result<()> {
    let relative = div((text("R"),)).with_inline_style(
        fixture_size(2.0, 1.0)
            .position(Position::Relative)
            .inset(cell_insets(Some(1.0), Some(9.0), Some(4.0), Some(2.0))),
    );
    let absolute = div((text("A"),)).with_inline_style(
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .position(Position::Absolute)
            .inset(cell_insets(Some(1.0), Some(3.0), Some(2.0), Some(2.0))),
    );
    let root = div((relative, absolute))
        .with_inline_style(
            fixture_size(12.0, 6.0)
                .position(Position::Relative)
                .overflow(Axes::all(Overflow::Visible)),
        )
        .into_view();

    let _terminal = render_view(root.as_view(), 12, 6)?;
    let rects = retained_child_rects(&root);

    assert_eq!(rects[0], Rect::new(2, 1, 2, 1));
    assert_eq!(rects[1], Rect::new(2, 1, 7, 3));
    Ok(())
}

/// Verifies absolute descendants select the nearest non-static containing block.
///
/// # Example Under Test
///
/// ```text
/// 20x10 static root
/// static 10x6 wrapper with margin-left: 4
/// absolute "A" at top: 1 and left: 1
/// relative 4x3 inner box with margin-left: 2
/// absolute "B" inside the inner box at top: 1 and left: 1
/// ```
///
/// # Assertions
///
/// - The direct absolute descendant skips the static wrapper and uses the root.
/// - The relative inner box remains positioned inside the wrapper.
/// - The nested absolute descendant uses the relative inner box.
/// - The static root acts as the terminal containing-block fallback.
#[test]
fn absolute_descendants_use_the_nearest_non_static_ancestor_or_root() -> Result<()> {
    let direct_absolute = div((text("A"),)).with_inline_style(
        fixture_size(1.0, 1.0)
            .position(Position::Absolute)
            .inset(cell_insets(Some(1.0), None, None, Some(1.0))),
    );
    let nested_absolute = div((text("B"),)).with_inline_style(
        fixture_size(1.0, 1.0)
            .position(Position::Absolute)
            .inset(cell_insets(Some(1.0), None, None, Some(1.0))),
    );
    let relative = div((nested_absolute,)).with_inline_style(
        fixture_size(4.0, 3.0)
            .position(Position::Relative)
            .margin(cell_insets(None, None, None, Some(2.0))),
    );
    let wrapper = div((direct_absolute, relative)).with_inline_style(
        fixture_size(10.0, 6.0).margin(cell_insets(None, None, None, Some(4.0))),
    );
    let root = div((wrapper,))
        .with_inline_style(fixture_size(20.0, 10.0).overflow(Axes::all(Overflow::Visible)))
        .into_view();

    let _terminal = render_view(root.as_view(), 20, 10)?;
    let root_div = root
        .downcast_ref::<DivView>()
        .expect("positioning fixture root should be a DivView");
    let wrapper_view = root_div.child_views()[0]
        .downcast_ref::<DivView>()
        .expect("static wrapper should be a DivView");
    let relative_view = wrapper_view.child_views()[1]
        .downcast_ref::<DivView>()
        .expect("relative fixture should be a DivView");

    assert_eq!(
        retained_rect(&root_div.child_views()[0]),
        Rect::new(4, 0, 10, 6)
    );
    assert_eq!(
        retained_rect(&wrapper_view.child_views()[0]),
        Rect::new(1, 1, 1, 1)
    );
    assert_eq!(
        retained_rect(&wrapper_view.child_views()[1]),
        Rect::new(6, 0, 4, 3)
    );
    assert_eq!(
        retained_rect(&relative_view.child_views()[0]),
        Rect::new(7, 1, 1, 1)
    );
    Ok(())
}

/// Verifies percentage insets recompute against resized containing blocks.
///
/// # Example Under Test
///
/// ```text
/// relative root: 100% x 100%
/// absolute 2x1 child: top 50%, left 25%
/// viewport: 20x10, then 40x20
/// ```
///
/// # Assertions
///
/// - The small viewport places the child at column five and row five.
/// - The large viewport places the child at column ten and row ten.
/// - The child's explicit size remains stable across both renders.
#[test]
fn percentage_insets_recompute_after_terminal_resize() -> Result<()> {
    let absolute = div((text("P"),)).with_inline_style(
        fixture_size(2.0, 1.0)
            .position(Position::Absolute)
            .inset(Edges::new(
                Length::percent(50.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto,
                Length::percent(25.0).into(),
            )),
    );
    let root = div((absolute,))
        .with_inline_style(
            TuiStyle::new()
                .box_sizing(BoxSizing::BorderBox)
                .position(Position::Relative)
                .size(LayoutSize::new(
                    Dimension::from(Length::percent(100.0)),
                    Dimension::from(Length::percent(100.0)),
                ))
                .overflow(Axes::all(Overflow::Visible)),
        )
        .into_view();

    let _small = render_view(root.as_view(), 20, 10)?;
    assert_eq!(retained_child_rects(&root)[0], Rect::new(5, 5, 2, 1));

    let _large = render_view(root.as_view(), 40, 20)?;
    assert_eq!(retained_child_rects(&root)[0], Rect::new(10, 10, 2, 1));
    Ok(())
}

/// Verifies fixed headers and footers leave flow and use viewport edge insets.
///
/// # Example Under Test
///
/// ```text
/// viewport: 12x5
/// fixed 12x1 header at top: 0 and left: 0
/// fixed 12x1 footer at bottom: 0 and left: 0
/// normal-flow rows "A" and "B"
/// ```
///
/// # Assertions
///
/// - The header occupies the terminal's first row.
/// - The footer occupies the terminal's last row.
/// - The second flow row remains at row one because neither fixed box reserves
///   flow space.
#[test]
fn fixed_header_and_footer_use_viewport_edges_without_reserving_flow() -> Result<()> {
    let header = div((text("HEADER"),)).with_inline_style(
        fixture_size(12.0, 1.0)
            .position(Position::Fixed)
            .inset(cell_insets(Some(0.0), None, None, Some(0.0))),
    );
    let footer = div((text("FOOTER"),)).with_inline_style(
        fixture_size(12.0, 1.0)
            .position(Position::Fixed)
            .inset(cell_insets(None, None, Some(0.0), Some(0.0))),
    );
    let root = div((text("A"), header, text("B"), footer))
        .with_inline_style(fixture_size(12.0, 5.0))
        .into_view();

    let terminal = render_view(root.as_view(), 12, 5)?;
    let lines = rendered_lines(&terminal);
    let rects = retained_child_rects(&root);

    assert_eq!(rects[1], Rect::new(0, 0, 12, 1));
    assert_eq!(rects[3], Rect::new(0, 4, 12, 1));
    assert_eq!(rects[2].y, 1);
    assert_eq!(lines[0], "HEADER      ");
    assert_eq!(lines[4], "FOOTER      ");
    Ok(())
}

/// Verifies fixed overlays ignore positioned ancestor containing blocks.
///
/// # Example Under Test
///
/// ```text
/// viewport: 20x6
/// relative 4x3 ancestor at margin-left: 4
/// fixed 3x2 overlay at top: 1 and right: 2
/// ```
///
/// # Assertions
///
/// - The fixed overlay resolves its right inset from the terminal viewport.
/// - The relative ancestor's origin and dimensions do not affect the overlay.
/// - The overlay paints at its retained viewport-relative coordinates.
#[test]
fn fixed_overlay_uses_terminal_viewport_instead_of_positioned_ancestor() -> Result<()> {
    let overlay = div((text("XYZ"),)).with_inline_style(
        fixture_size(3.0, 2.0)
            .position(Position::Fixed)
            .inset(cell_insets(Some(1.0), Some(2.0), None, None)),
    );
    let ancestor = div((overlay,)).with_inline_style(
        fixture_size(4.0, 3.0)
            .position(Position::Relative)
            .margin(cell_insets(None, None, None, Some(4.0))),
    );
    let root = div((ancestor,))
        .with_inline_style(fixture_size(20.0, 6.0))
        .into_view();

    let terminal = render_view(root.as_view(), 20, 6)?;
    let root_div = root
        .downcast_ref::<DivView>()
        .expect("fixed overlay root should be a DivView");
    let ancestor = root_div.child_views()[0]
        .downcast_ref::<DivView>()
        .expect("fixed overlay ancestor should be a DivView");

    assert_eq!(
        retained_rect(&ancestor.child_views()[0]),
        Rect::new(15, 1, 3, 2)
    );
    assert_eq!(&rendered_lines(&terminal)[1][15..18], "XYZ");
    Ok(())
}

/// Verifies fixed percentage insets recompute against terminal resize.
///
/// # Example Under Test
///
/// ```text
/// fixed 2x1 child: top 50%, left 25%
/// viewport: 20x10, then 40x20
/// ```
///
/// # Assertions
///
/// - The small viewport places the fixed child at column five and row five.
/// - The large viewport places the fixed child at column ten and row ten.
/// - The fixed child's explicit size remains stable across both renders.
#[test]
fn fixed_percentage_insets_recompute_after_terminal_resize() -> Result<()> {
    let fixed = div((text("F"),)).with_inline_style(
        fixture_size(2.0, 1.0)
            .position(Position::Fixed)
            .inset(Edges::new(
                Length::percent(50.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto,
                Length::percent(25.0).into(),
            )),
    );
    let root = div((fixed,))
        .with_inline_style(fixture_size(4.0, 2.0))
        .into_view();

    let _small = render_view(root.as_view(), 20, 10)?;
    assert_eq!(retained_child_rects(&root)[0], Rect::new(5, 5, 2, 1));

    let _large = render_view(root.as_view(), 40, 20)?;
    assert_eq!(retained_child_rects(&root)[0], Rect::new(10, 10, 2, 1));
    Ok(())
}

/// Verifies fixed descendants remain stable while a nested ancestor scrolls.
///
/// # Example Under Test
///
/// ```text
/// viewport: 10x5
/// nested 8x3 scrolling div with four flow rows
/// fixed 1x1 descendant at top: 0 and right: 0
/// scroll nested div down two rows
/// ```
///
/// # Assertions
///
/// - The nested container consumes the requested scroll.
/// - The fixed descendant retains the same viewport-relative rectangle.
/// - The fixed marker remains painted in the terminal's top-right cell.
#[test]
fn fixed_descendant_ignores_nested_ancestor_scrolling() -> Result<()> {
    let fixed = div((text("F"),)).with_inline_style(
        fixture_size(1.0, 1.0)
            .position(Position::Fixed)
            .inset(cell_insets(Some(0.0), Some(0.0), None, None)),
    );
    let scroller = div((text("one"), fixed, text("two"), text("three"), text("four")))
        .with_inline_style(
            fixture_size(8.0, 3.0).overflow(Axes::new(Overflow::Hidden, Overflow::Auto)),
        );
    let mut root = div((scroller,)).with_inline_style(fixture_size(10.0, 5.0));
    let mut terminal = Terminal::new(TestBackend::new(10, 5))?;

    draw_view(&mut terminal, &root)?;
    let root_view = root
        .child_views()
        .first()
        .and_then(|child| child.downcast_ref::<DivView>())
        .expect("nested scrolling fixture should contain a DivView");
    let before = retained_rect(&root_view.child_views()[1]);
    assert_eq!(before, Rect::new(9, 0, 1, 1));
    assert_eq!(terminal.backend().buffer()[(9, 0)].symbol(), "F");

    assert!(root.__scroll_overflowing_at_position(0, 1, Axes::new(0, 2)));
    draw_view(&mut terminal, &root)?;
    let root_view = root
        .child_views()
        .first()
        .and_then(|child| child.downcast_ref::<DivView>())
        .expect("nested scrolling fixture should retain its DivView");

    assert_eq!(retained_rect(&root_view.child_views()[1]), before);
    assert_eq!(terminal.backend().buffer()[(9, 0)].symbol(), "F");
    Ok(())
}

/// Verifies fixed boxes escape ancestor clips but remain terminal-clipped.
///
/// # Example Under Test
///
/// ```text
/// viewport: 10x4
/// hidden-overflow ancestor: 3x2
/// fixed 5x1 "FIXED" at top: 2 and left: 6
/// ```
///
/// # Assertions
///
/// - The fixed box paints outside the ancestor's hidden-overflow rectangle.
/// - The terminal displays only the four columns that fit in its viewport.
/// - Retained clipping uses the terminal viewport instead of the ancestor.
#[test]
fn fixed_box_escapes_ancestor_clip_and_clips_to_terminal_viewport() -> Result<()> {
    let fixed = div((text("FIXED"),)).with_inline_style(
        fixture_size(5.0, 1.0)
            .position(Position::Fixed)
            .inset(cell_insets(Some(2.0), None, None, Some(6.0))),
    );
    let clipped = div((fixed,))
        .with_inline_style(fixture_size(3.0, 2.0).overflow(Axes::all(Overflow::Hidden)));
    let root = div((clipped,))
        .with_inline_style(fixture_size(10.0, 4.0))
        .into_view();

    let terminal = render_view(root.as_view(), 10, 4)?;
    let root_div = root
        .downcast_ref::<DivView>()
        .expect("fixed clipping root should be a DivView");
    let clipped = root_div.child_views()[0]
        .downcast_ref::<DivView>()
        .expect("fixed clipping ancestor should be a DivView");
    let geometry = clipped.child_views()[0]
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("fixed clipping fixture should retain geometry");

    assert_eq!(geometry.border_box, Rect::new(6, 2, 5, 1));
    assert_eq!(geometry.clip, Rect::new(0, 0, 10, 4));
    assert_eq!(&rendered_lines(&terminal)[2][6..10], "FIXE");
    Ok(())
}
