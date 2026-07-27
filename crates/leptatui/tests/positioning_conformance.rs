//! Static, relative, absolute, fixed, and sticky positioning conformance tests.
//!
//! These tests exercise normal-flow participation, positioned subtree
//! translation, containing-block and scrollport selection, opposing edge
//! resolution, percentage insets, scrolling, and resize recomputation through
//! the public style API.

use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use leptatui::__private::FocusedControl;
use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

mod support;

use support::{
    draw_view, fixture_size, key, render_view, rendered_lines, rendered_text, retained_child_rects,
};

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

/// Creates a mouse event at one absolute terminal coordinate.
///
/// # Arguments
///
/// * `kind` — Mouse action represented by the event.
/// * `column` — Zero-based terminal column.
/// * `row` — Zero-based terminal row.
///
/// # Returns
///
/// An [`Event`] containing the requested mouse action.
fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Returns button focus states in logical source order.
///
/// # Arguments
///
/// * `view` — Root view whose logical descendants are inspected.
///
/// # Returns
///
/// A [`Vec`] containing one focus flag per button.
fn button_focuses(view: &dyn View) -> Vec<bool> {
    let mut focuses = Vec::new();
    collect_button_focuses(view, &mut focuses);
    focuses
}

/// Appends logical button focus states from one view subtree.
///
/// # Arguments
///
/// * `view` — Current view inspected for button metadata and children.
/// * `focuses` — Output vector receiving source-ordered focus flags.
fn collect_button_focuses(view: &dyn View, focuses: &mut Vec<bool>) {
    if let Some(button) = view.as_any().downcast_ref::<ButtonView>() {
        focuses.push(
            button
                .style_metadata()
                .expect("buttons should expose style metadata")
                .is_focused(),
        );
    }
    for child in view.children() {
        collect_button_focuses(child.as_view(), focuses);
    }
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

/// Verifies a top-inset sticky box follows flow before pinning to its scrollport.
///
/// # Example Under Test
///
/// ```text
/// 8x4 vertical scrollport
/// "A", sticky 8x1 "S" with top: 0, then four flow rows
/// scroll down two rows
/// ```
///
/// # Assertions
///
/// - The sticky box initially paints on its normal-flow row.
/// - Scrolling past that row pins the sticky box to the scrollport top.
/// - Its retained normal-flow row and the scrollable content extent do not
///   change.
#[test]
fn top_sticky_follows_flow_then_pins_without_changing_extents() -> Result<()> {
    let sticky = div((text("S"),)).with_inline_style(
        fixture_size(8.0, 1.0)
            .position(Position::Sticky)
            .inset(cell_insets(Some(0.0), None, None, None))
            .z_index(ZIndex::Integer(1)),
    );
    let mut root = div((
        text("A"),
        sticky,
        text("B"),
        text("C"),
        text("D"),
        text("E"),
    ))
    .with_inline_style(
        fixture_size(8.0, 4.0).overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
    );
    let mut terminal = Terminal::new(TestBackend::new(8, 4))?;

    draw_view(&mut terminal, &root)?;
    assert!(rendered_lines(&terminal)[1].starts_with('S'));
    assert_eq!(retained_rect(&root.child_views()[1]).y, 1);
    assert_eq!(
        root.style_metadata()
            .expect("sticky fixture should expose metadata")
            .content_extent()
            .height,
        6
    );

    assert!(root.__scroll_overflowing_at_position(0, 1, Axes::new(0, 2)));
    draw_view(&mut terminal, &root)?;

    assert!(rendered_lines(&terminal)[0].starts_with('S'));
    assert_eq!(retained_rect(&root.child_views()[1]).y, 1);
    assert_eq!(
        root.style_metadata()
            .expect("sticky fixture should retain metadata")
            .content_extent()
            .height,
        6
    );
    Ok(())
}

/// Verifies a bottom-inset sticky box clamps only while crossing its threshold.
///
/// # Example Under Test
///
/// ```text
/// 8x4 vertical scrollport
/// four flow rows, sticky 8x1 "S" with bottom: 0, then one flow row
/// render at the top, then scroll down two rows
/// ```
///
/// # Assertions
///
/// - The initially offscreen normal box clamps to the scrollport bottom.
/// - After scrolling its normal-flow row inside the viewport, the box follows
///   that row upward.
/// - The retained normal-flow row remains unchanged.
#[test]
fn bottom_sticky_clamps_at_the_scrollport_end_threshold() -> Result<()> {
    let sticky = div((text("S"),)).with_inline_style(
        fixture_size(8.0, 1.0)
            .position(Position::Sticky)
            .inset(cell_insets(None, None, Some(0.0), None))
            .z_index(ZIndex::Integer(1)),
    );
    let mut root = div((
        text("A"),
        text("B"),
        text("C"),
        text("D"),
        sticky,
        text("E"),
    ))
    .with_inline_style(
        fixture_size(8.0, 4.0).overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
    );
    let mut terminal = Terminal::new(TestBackend::new(8, 4))?;

    draw_view(&mut terminal, &root)?;
    assert!(rendered_lines(&terminal)[3].starts_with('S'));
    assert_eq!(retained_rect(&root.child_views()[4]).y, 4);

    assert!(root.__scroll_overflowing_at_position(0, 1, Axes::new(0, 2)));
    draw_view(&mut terminal, &root)?;

    assert!(rendered_lines(&terminal)[2].starts_with('S'));
    assert_eq!(retained_rect(&root.child_views()[4]).y, 4);
    Ok(())
}

/// Verifies sticky boxes use the nearest nested scrollport.
///
/// # Example Under Test
///
/// ```text
/// 10x4 outer scrollport
/// two outer flow rows
/// nested 8x3 scrollport containing sticky "S" with top: 1 and four more rows
/// two trailing outer rows
/// scroll the outer container to the bottom, then scroll inside the nested one
/// ```
///
/// # Assertions
///
/// - Outer scrolling partially clips the nested scrollport above the terminal.
/// - The sticky marker uses the nested scrollport's signed origin and paints at
///   row zero rather than the terminal-relative inset row.
/// - The nested container consumes a later scroll at its visible position.
/// - The marker remains constrained after nested scrolling.
#[test]
fn sticky_uses_the_nearest_nested_scrollport() -> Result<()> {
    let sticky = div((text("S"),)).with_inline_style(
        fixture_size(8.0, 1.0)
            .position(Position::Sticky)
            .inset(cell_insets(Some(1.0), None, None, None))
            .z_index(ZIndex::Integer(1)),
    );
    let nested = div((sticky, text("A"), text("B"), text("C"), text("D"))).with_inline_style(
        fixture_size(8.0, 3.0).overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
    );
    let mut root = div((
        text("OUTER-A"),
        text("OUTER-B"),
        nested,
        text("OUTER-C"),
        text("OUTER-D"),
    ))
    .with_inline_style(
        fixture_size(10.0, 4.0).overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
    );
    let mut terminal = Terminal::new(TestBackend::new(10, 4))?;

    draw_view(&mut terminal, &root)?;
    assert!(root.__scroll_first_overflowing_to_bottom());
    draw_view(&mut terminal, &root)?;
    assert!(rendered_lines(&terminal)[0].starts_with('S'));

    assert!(root.__scroll_overflowing_at_position(0, 0, Axes::new(0, 2)));
    draw_view(&mut terminal, &root)?;

    assert!(rendered_lines(&terminal)[0].starts_with('S'));
    Ok(())
}

/// Verifies an oversized sticky box uses start-edge precedence safely.
///
/// # Example Under Test
///
/// ```text
/// 8x3 vertical scrollport
/// sticky 8x4 "S" with top: 0 and bottom: 0
/// two trailing flow rows
/// scroll to the bottom
/// ```
///
/// # Assertions
///
/// - Conflicting top and bottom constraints keep the sticky start at row zero.
/// - The oversized box remains clipped by the scrollport.
/// - Normal-flow content height and maximum scroll offset include the original
///   sticky dimensions.
#[test]
fn oversized_sticky_prefers_the_start_edge_without_corrupting_extents() -> Result<()> {
    let sticky = div((text("S"),)).with_inline_style(
        fixture_size(8.0, 4.0)
            .position(Position::Sticky)
            .inset(cell_insets(Some(0.0), None, Some(0.0), None))
            .z_index(ZIndex::Integer(1)),
    );
    let mut root = div((sticky, text("A"), text("B"))).with_inline_style(
        fixture_size(8.0, 3.0).overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
    );
    let mut terminal = Terminal::new(TestBackend::new(8, 3))?;

    draw_view(&mut terminal, &root)?;
    assert!(root.__scroll_first_overflowing_to_bottom());
    draw_view(&mut terminal, &root)?;

    let metadata = root
        .style_metadata()
        .expect("oversized sticky fixture should expose metadata");
    assert!(rendered_lines(&terminal)[0].starts_with('S'));
    assert_eq!(metadata.content_extent().height, 6);
    assert_eq!(metadata.max_scroll_offset(), 3);
    assert_eq!(retained_rect(&root.child_views()[0]).y, 0);
    Ok(())
}

/// Verifies percentage sticky insets recompute after terminal resize.
///
/// # Example Under Test
///
/// ```text
/// percentage-sized vertical scrollport with twelve flow rows
/// sticky second row with top: 50%
/// render at 10x4, scroll two rows, then render at 20x8
/// ```
///
/// # Assertions
///
/// - The small scrollport clamps the sticky marker at row two.
/// - The resized scrollport clamps the same marker at row four.
/// - The existing scroll offset survives the resize.
#[test]
fn percentage_sticky_inset_recomputes_after_terminal_resize() -> Result<()> {
    let sticky = div((text("S"),)).with_inline_style(
        fixture_size(10.0, 1.0)
            .position(Position::Sticky)
            .inset(Edges::new(
                Length::percent(50.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto,
                LengthAuto::Auto,
            ))
            .z_index(ZIndex::Integer(1)),
    );
    let mut root = div((
        text("A"),
        sticky,
        text("B"),
        text("C"),
        text("D"),
        text("E"),
        text("F"),
        text("G"),
        text("H"),
        text("I"),
        text("J"),
        text("K"),
    ))
    .with_inline_style(
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0)),
            ))
            .overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
    );
    let mut small = Terminal::new(TestBackend::new(10, 4))?;

    draw_view(&mut small, &root)?;
    assert!(root.__scroll_overflowing_at_position(0, 1, Axes::new(0, 2)));
    draw_view(&mut small, &root)?;
    assert!(rendered_lines(&small)[2].starts_with('S'));

    let mut large = Terminal::new(TestBackend::new(20, 8))?;
    draw_view(&mut large, &root)?;

    assert!(rendered_lines(&large)[4].starts_with('S'));
    assert_eq!(
        root.style_metadata()
            .expect("resized sticky fixture should expose metadata")
            .scroll_offset(),
        2
    );
    Ok(())
}

/// Verifies focus scrolling preserves a sticky header's constrained position.
///
/// # Example Under Test
///
/// ```text
/// 12x4 vertical scrollport
/// top-sticky "S", three flow rows, focused button "Target", trailing row
/// Tab, then render
/// ```
///
/// # Assertions
///
/// - Tab focuses the target button.
/// - Rendering scrolls the target into view.
/// - The sticky marker remains painted on the scrollport's top row.
/// - The focused button label remains visible below the sticky marker.
#[test]
fn focus_scrolling_keeps_the_sticky_header_constrained() -> Result<()> {
    let sticky = div((text("S"),)).with_inline_style(
        fixture_size(12.0, 1.0)
            .position(Position::Sticky)
            .inset(cell_insets(Some(0.0), None, None, None))
            .z_index(ZIndex::Integer(1)),
    );
    let mut root = div((
        sticky,
        text("A"),
        text("B"),
        text("C"),
        button("Target"),
        text("D"),
    ))
    .with_inline_style(
        fixture_size(12.0, 4.0).overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
    );
    let mut terminal = Terminal::new(TestBackend::new(12, 4))?;

    View::handle_event(&mut root, key(crossterm::event::KeyCode::Tab))?;
    draw_view(&mut terminal, &root)?;

    assert!(
        root.style_metadata()
            .expect("focus sticky fixture should expose metadata")
            .scroll_offset()
            > 0
    );
    assert!(rendered_lines(&terminal)[0].starts_with('S'));
    assert!(rendered_text(&terminal).contains("Target"));
    Ok(())
}

/// Verifies positioned layers surround normal flow in deterministic order.
///
/// # Example Under Test
///
/// ```text
/// four 1x1 overlapping roots:
/// negative positioned after normal flow
/// automatic positioned before static z-index 100
/// explicit zero before automatic positioned
/// positive z-index 2 before positive z-index 1
/// ```
///
/// # Assertions
///
/// - Normal flow paints over a later negative positioned box.
/// - Automatic positioned content paints over a later static box whose
///   authored z-index is ignored.
/// - Automatic and explicit zero levels use source order.
/// - Larger positive levels paint over smaller levels regardless of source
///   order.
#[test]
fn positioned_layers_surround_normal_flow_and_break_ties_by_source_order() -> Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let root_style = fixture_size(1.0, 1.0)
        .position(Position::Relative)
        .overflow(Axes::all(Overflow::Visible));
    let positioned = |label, z_index| {
        text(label).with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset)
                .z_index(z_index),
        )
    };

    let negative = div((text("N"), positioned("X", ZIndex::Integer(-1))))
        .with_inline_style(root_style.clone())
        .into_view();
    assert_eq!(
        rendered_lines(&render_view(negative.as_view(), 1, 1)?)[0],
        "N"
    );

    let automatic = text("A").with_inline_style(
        fixture_size(1.0, 1.0)
            .position(Position::Absolute)
            .inset(inset),
    );
    let static_with_z_index =
        text("S").with_inline_style(TuiStyle::new().z_index(ZIndex::Integer(100)));
    let normal_then_positioned = div((automatic, static_with_z_index))
        .with_inline_style(root_style.clone())
        .into_view();
    assert_eq!(
        rendered_lines(&render_view(normal_then_positioned.as_view(), 1, 1)?)[0],
        "A"
    );

    let zero_then_auto = div((
        positioned("Z", ZIndex::Integer(0)),
        text("A").with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset),
        ),
    ))
    .with_inline_style(root_style.clone())
    .into_view();
    assert_eq!(
        rendered_lines(&render_view(zero_then_auto.as_view(), 1, 1)?)[0],
        "A"
    );

    let positive = div((
        positioned("H", ZIndex::Integer(2)),
        positioned("L", ZIndex::Integer(1)),
    ))
    .with_inline_style(root_style)
    .into_view();
    assert_eq!(
        rendered_lines(&render_view(positive.as_view(), 1, 1)?)[0],
        "H"
    );
    Ok(())
}

/// Verifies normal-flow overlaps retain source-order painting.
///
/// # Example Under Test
///
/// ```text
/// 1x1 grid
/// static "A" in grid area 1 / 1
/// static "B" in the same grid area
/// ```
///
/// # Assertions
///
/// - Both static children occupy the same retained grid cell.
/// - The later static child paints over the earlier child.
#[test]
fn overlapping_normal_flow_grid_items_paint_in_source_order() -> Result<()> {
    let placement = TuiStyle::new()
        .grid_row(GridLine::new(
            GridPlacement::line(1),
            GridPlacement::line(2),
        ))
        .grid_column(GridLine::new(
            GridPlacement::line(1),
            GridPlacement::line(2),
        ));
    let root = div((
        text("A").with_inline_style(placement.clone()),
        text("B").with_inline_style(placement),
    ))
    .with_inline_style(
        fixture_size(1.0, 1.0)
            .display(Display::Grid)
            .grid_template_columns(vec![GridTemplateTrack::from(GridTrackSize::from(
                Length::cells(1.0),
            ))])
            .grid_template_rows(vec![GridTemplateTrack::from(GridTrackSize::from(
                Length::cells(1.0),
            ))]),
    )
    .into_view();

    let terminal = render_view(root.as_view(), 1, 1)?;
    let rects = retained_child_rects(&root);

    assert_eq!(rects, vec![Rect::new(0, 0, 1, 1); 2]);
    assert_eq!(rendered_lines(&terminal)[0], "B");
    Ok(())
}

/// Verifies explicit nested contexts keep descendant levels locally isolated.
///
/// # Example Under Test
///
/// ```text
/// root 1: parent z-index -1 containing child z-index 100, sibling z-index 0
/// root 2: sibling z-index 0, parent z-index 1 containing child z-index -100
/// ```
///
/// # Assertions
///
/// - A high descendant cannot escape its negative parent context.
/// - A negative descendant remains above a sibling behind its positive parent
///   context.
#[test]
fn explicit_nested_stacking_contexts_are_atomic_between_siblings() -> Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let root_style = fixture_size(1.0, 1.0).position(Position::Relative);
    let context = |label, parent_level, child_level| {
        div((text(label).with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset)
                .z_index(ZIndex::Integer(child_level)),
        ),))
        .with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset)
                .z_index(ZIndex::Integer(parent_level)),
        )
    };
    let sibling = || {
        text("M").with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset)
                .z_index(ZIndex::Integer(0)),
        )
    };

    let trapped_high = div((context("H", -1, 100), sibling()))
        .with_inline_style(root_style.clone())
        .into_view();
    assert_eq!(
        rendered_lines(&render_view(trapped_high.as_view(), 1, 1)?)[0],
        "M"
    );

    let retained_low = div((sibling(), context("L", 1, -100)))
        .with_inline_style(root_style)
        .into_view();
    assert_eq!(
        rendered_lines(&render_view(retained_low.as_view(), 1, 1)?)[0],
        "L"
    );
    Ok(())
}

/// Verifies context chrome precedes negative children and clipping still applies.
///
/// # Example Under Test
///
/// ```text
/// 3x3 blue relative box with borders
/// 4x1 red absolute child at z-index -1
/// hidden overflow
/// terminal size: 4x3
/// ```
///
/// # Assertions
///
/// - The parent border remains painted around its content box.
/// - The negative child paints over the parent's content background.
/// - The child remains clipped at the parent's right border.
#[test]
fn negative_children_paint_over_context_background_without_escaping_clip() -> Result<()> {
    let child = div((text("XXXX"),)).with_inline_style(
        fixture_size(4.0, 1.0)
            .background(Color::Red)
            .position(Position::Absolute)
            .inset(cell_insets(Some(0.0), None, None, Some(0.0)))
            .z_index(ZIndex::Integer(-1)),
    );
    let root = div((child,))
        .with_inline_style(
            fixture_size(3.0, 3.0)
                .background(Color::Blue)
                .borders(Borders::ALL)
                .position(Position::Relative)
                .overflow(Axes::all(Overflow::Hidden)),
        )
        .into_view();

    let terminal = render_view(root.as_view(), 4, 3)?;
    let lines = rendered_lines(&terminal);

    assert_eq!(lines[0], "┌─┐ ");
    assert_eq!(lines[1], "│X│ ");
    assert_eq!(terminal.backend().buffer()[(1, 1)].bg, Color::Red,);
    assert_eq!(terminal.backend().buffer()[(3, 1)].bg, Color::Reset);
    Ok(())
}

/// Verifies fixed boxes use viewport stacking levels instead of traversal order.
///
/// # Example Under Test
///
/// ```text
/// fixed "H" at z-index 2
/// fixed "L" at z-index 1 later in source order
/// both at viewport row 0, column 0
/// ```
///
/// # Assertions
///
/// - The earlier higher-level fixed box paints over the later lower-level box.
#[test]
fn fixed_descendants_paint_in_viewport_stacking_order() -> Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let fixed = |label, level| {
        text(label).with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Fixed)
                .inset(inset)
                .z_index(ZIndex::Integer(level)),
        )
    };
    let root = div((fixed("H", 2), fixed("L", 1)))
        .with_inline_style(fixture_size(1.0, 1.0))
        .into_view();

    let terminal = render_view(root.as_view(), 1, 1)?;

    assert_eq!(rendered_lines(&terminal)[0], "H");
    Ok(())
}

/// Verifies pointer targeting follows the final global fixed paint pass.
///
/// # Example Under Test
///
/// ```text
/// relative 6x3 root
/// first branch contains a fixed button at viewport origin
/// later branch contains an absolute button at the same origin
/// MouseMoved(2, 1)
/// ```
///
/// # Assertions
///
/// - Pointer movement focuses the fixed button.
/// - Logical button focus state remains source-ordered.
///
/// # Why
///
/// Direct-child ordering cannot represent fixed descendants that escape an
/// earlier logical branch and paint during the deferred viewport pass.
#[test]
fn pointer_targeting_uses_global_fixed_paint_order() -> Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let fixed = button("Fixed").with_inline_style(
        fixture_size(6.0, 3.0)
            .position(Position::Fixed)
            .inset(inset),
    );
    let absolute = button("Under").with_inline_style(
        fixture_size(6.0, 3.0)
            .position(Position::Absolute)
            .inset(inset)
            .z_index(ZIndex::Integer(10)),
    );
    let mut root = div((div((fixed,)), absolute))
        .with_inline_style(fixture_size(6.0, 3.0).position(Position::Relative))
        .into_view();

    let _terminal = render_view(root.as_view(), 6, 3)?;

    root.handle_event(mouse(MouseEventKind::Moved, 2, 1))?;

    assert_eq!(button_focuses(root.as_view()), vec![true, false]);
    Ok(())
}

/// Verifies clipped positioned controls expose only painted terminal cells.
///
/// # Example Under Test
///
/// ```text
/// 4x2 hidden-overflow relative root
/// absolute 4x2 button at left: 3
/// MouseMoved(5, 1), then MouseMoved(3, 1)
/// ```
///
/// # Assertions
///
/// - A pointer outside the root clip does not focus the button.
/// - The button becomes focusable through its single visible terminal column.
#[test]
fn clipped_positioned_control_uses_final_visible_hit_area() -> Result<()> {
    let clipped = button("Wide").with_inline_style(
        fixture_size(4.0, 2.0)
            .position(Position::Absolute)
            .inset(cell_insets(Some(0.0), None, None, Some(3.0))),
    );
    let mut root = div((clipped,))
        .with_inline_style(
            fixture_size(4.0, 2.0)
                .position(Position::Relative)
                .overflow(Axes::all(Overflow::Hidden)),
        )
        .into_view();
    let _terminal = render_view(root.as_view(), 8, 2)?;

    assert!(!root.__focus_control_at_position(5, 1));
    assert!(root.__focus_control_at_position(3, 1));
    assert_eq!(button_focuses(root.as_view()), vec![true]);
    Ok(())
}

/// Verifies a pinned focused control does not restore its normal-flow row.
///
/// # Example Under Test
///
/// ```text
/// 8x3 scrollport
/// top-sticky button followed by three flow rows
/// ScrollDown, MouseMoved(1, 0), render
/// ```
///
/// # Assertions
///
/// - Scrolling advances the parent offset while the button remains pinned.
/// - Pointer movement focuses the painted sticky button.
/// - Focus visibility preserves the existing scroll offset.
///
/// # Why
///
/// Focus scrolling must use the sticky box's constrained paint coordinate
/// instead of its superseded normal-flow coordinate.
#[test]
fn sticky_focus_visibility_uses_constrained_paint_geometry() -> Result<()> {
    let sticky = button("Pin").with_inline_style(
        fixture_size(8.0, 1.0)
            .position(Position::Sticky)
            .inset(cell_insets(Some(0.0), None, None, None))
            .z_index(ZIndex::Integer(1)),
    );
    let mut root = div((sticky, text("A"), text("B"), text("C")))
        .with_inline_style(
            fixture_size(8.0, 3.0).overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
        )
        .into_view();
    let mut terminal = Terminal::new(TestBackend::new(8, 3))?;

    draw_view(&mut terminal, root.as_view())?;
    root.handle_event(mouse(MouseEventKind::ScrollDown, 1, 0))?;
    draw_view(&mut terminal, root.as_view())?;
    assert_eq!(
        root.style_metadata()
            .expect("sticky focus root should expose metadata")
            .scroll_offset(),
        1
    );

    root.handle_event(mouse(MouseEventKind::Moved, 1, 0))?;
    draw_view(&mut terminal, root.as_view())?;

    assert_eq!(button_focuses(root.as_view()), vec![true]);
    assert_eq!(
        root.style_metadata()
            .expect("sticky focus root should expose metadata")
            .scroll_offset(),
        1
    );
    Ok(())
}

/// Verifies fixed focus does not scroll its logical overflow ancestors.
///
/// # Example Under Test
///
/// ```text
/// 8x3 scrollport with four flow rows
/// fixed button at viewport origin
/// ScrollDown, MouseMoved(1, 0), render
/// ```
///
/// # Assertions
///
/// - Wheel input advances the root scroll offset.
/// - Pointer movement focuses the fixed button.
/// - Focus visibility leaves the logical ancestor at its current offset.
///
/// # Why
///
/// Fixed boxes use the terminal viewport and must not be interpreted as
/// offscreen descendants of a scrolled logical parent.
#[test]
fn fixed_focus_does_not_scroll_logical_ancestors() -> Result<()> {
    let fixed = button("Pin").with_inline_style(
        fixture_size(8.0, 1.0)
            .position(Position::Fixed)
            .inset(cell_insets(Some(0.0), None, None, Some(0.0))),
    );
    let mut root = div((fixed, text("A"), text("B"), text("C"), text("D")))
        .with_inline_style(
            fixture_size(8.0, 3.0).overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
        )
        .into_view();
    let mut terminal = Terminal::new(TestBackend::new(8, 3))?;

    draw_view(&mut terminal, root.as_view())?;
    root.handle_event(mouse(MouseEventKind::ScrollDown, 1, 0))?;
    draw_view(&mut terminal, root.as_view())?;
    assert_eq!(
        root.style_metadata()
            .expect("fixed focus root should expose metadata")
            .scroll_offset(),
        1
    );

    root.handle_event(mouse(MouseEventKind::Moved, 1, 0))?;
    draw_view(&mut terminal, root.as_view())?;

    assert_eq!(button_focuses(root.as_view()), vec![true]);
    assert_eq!(
        root.style_metadata()
            .expect("fixed focus root should expose metadata")
            .scroll_offset(),
        1
    );
    Ok(())
}

/// Verifies positioned standalone and rich-text links retain terminal hit areas.
///
/// # Example Under Test
///
/// ```text
/// relative 10x4 root
/// absolute link at top: 2, left: 4
/// MouseMoved(0, 0), then MouseMoved(4, 2)
/// absolute Markdown link at top: 1, left: 2
/// MouseMoved(2, 1)
/// ```
///
/// # Assertions
///
/// - The link does not focus at its unpositioned origin.
/// - The standalone link focuses at its final absolute terminal coordinate.
/// - The embedded Markdown link focuses at its final absolute coordinate.
#[test]
fn positioned_link_areas_use_final_terminal_coordinates() -> Result<()> {
    let positioned = link("Docs", "https://example.com").with_inline_style(
        fixture_size(4.0, 1.0)
            .position(Position::Absolute)
            .inset(cell_insets(Some(2.0), None, None, Some(4.0))),
    );
    let mut root = div((positioned,))
        .with_inline_style(fixture_size(10.0, 4.0).position(Position::Relative))
        .into_view();
    let _terminal = render_view(root.as_view(), 10, 4)?;

    assert!(!root.__focus_control_at_position(0, 0));
    assert!(root.__focus_control_at_position(4, 2));
    assert_eq!(root.__focused_control(), Some(FocusedControl::Link));

    let positioned_markdown = markdown("[Docs](https://example.com)").with_inline_style(
        fixture_size(8.0, 2.0)
            .position(Position::Absolute)
            .inset(cell_insets(Some(1.0), None, None, Some(2.0))),
    );
    let mut markdown_root = div((positioned_markdown,))
        .with_inline_style(fixture_size(10.0, 4.0).position(Position::Relative))
        .into_view();
    let _terminal = render_view(markdown_root.as_view(), 10, 4)?;

    assert!(markdown_root.__focus_control_at_position(2, 1));
    assert_eq!(
        markdown_root.__focused_control(),
        Some(FocusedControl::Link)
    );
    Ok(())
}

/// Verifies a positioned editor publishes its final absolute cursor coordinate.
///
/// # Example Under Test
///
/// ```text
/// relative 10x5 root
/// focused 6x3 absolute input("A") at top: 1, left: 3
/// ```
///
/// # Assertions
///
/// - The input paints at the positioned rectangle.
/// - The terminal cursor appears after `A` inside the positioned content box.
#[test]
fn positioned_editor_cursor_uses_final_terminal_coordinates() -> Result<()> {
    let positioned = input("A").with_focus(true).with_inline_style(
        fixture_size(6.0, 3.0)
            .position(Position::Absolute)
            .inset(cell_insets(Some(1.0), None, None, Some(3.0))),
    );
    let root = div((positioned,))
        .with_inline_style(fixture_size(10.0, 5.0).position(Position::Relative))
        .into_view();
    let mut terminal = render_view(root.as_view(), 10, 5)?;

    assert_eq!(rendered_lines(&terminal)[2].chars().nth(4), Some('A'));
    terminal.backend_mut().assert_cursor_position((5, 2));
    Ok(())
}

/// Verifies wheel targeting follows the final global fixed paint pass.
///
/// # Example Under Test
///
/// ```text
/// fixed 6x3 scroller in an early branch
/// absolute 6x3 scroller later in source order at the same coordinates
/// ScrollDown(1, 1)
/// ```
///
/// # Assertions
///
/// - The fixed scroller consumes the wheel delta.
/// - The covered absolute scroller retains its original offset.
#[test]
fn wheel_targeting_uses_frontmost_global_paint_order() -> Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let scroller = |position| {
        div((text("one"), text("two"), text("three"), text("four"))).with_inline_style(
            fixture_size(6.0, 3.0)
                .position(position)
                .inset(inset)
                .overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
        )
    };
    let mut root = div((
        div((scroller(Position::Fixed),)),
        scroller(Position::Absolute),
    ))
    .with_inline_style(fixture_size(6.0, 3.0).position(Position::Relative))
    .into_view();
    let _terminal = render_view(root.as_view(), 6, 3)?;

    root.handle_event(mouse(MouseEventKind::ScrollDown, 1, 1))?;

    let root_div = root
        .downcast_ref::<DivView>()
        .expect("wheel fixture root should be a DivView");
    let fixed_wrapper = root_div.child_views()[0]
        .downcast_ref::<DivView>()
        .expect("wheel fixture wrapper should be a DivView");
    let fixed_metadata = fixed_wrapper.child_views()[0]
        .style_metadata()
        .expect("fixed scroller should expose metadata");
    let absolute_metadata = root_div.child_views()[1]
        .style_metadata()
        .expect("absolute scroller should expose metadata");

    assert_eq!(fixed_metadata.scroll_offset(), 1);
    assert_eq!(absolute_metadata.scroll_offset(), 0);
    Ok(())
}
