//! Sticky positioning and scrollport conformance tests.

use super::*;

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
fn top_sticky_follows_flow_then_pins_without_changing_extents() -> leptatui::app::Result<()> {
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
fn bottom_sticky_clamps_at_the_scrollport_end_threshold() -> leptatui::app::Result<()> {
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
fn sticky_uses_the_nearest_nested_scrollport() -> leptatui::app::Result<()> {
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
fn oversized_sticky_prefers_the_start_edge_without_corrupting_extents() -> leptatui::app::Result<()>
{
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
fn percentage_sticky_inset_recomputes_after_terminal_resize() -> leptatui::app::Result<()> {
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
fn focus_scrolling_keeps_the_sticky_header_constrained() -> leptatui::app::Result<()> {
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
