//! Positioned input, focus, pointer, and hit-geometry conformance tests.

use super::*;

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
fn clipped_positioned_control_uses_final_visible_hit_area() -> leptatui::app::Result<()> {
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
fn sticky_focus_visibility_uses_constrained_paint_geometry() -> leptatui::app::Result<()> {
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
fn fixed_focus_does_not_scroll_logical_ancestors() -> leptatui::app::Result<()> {
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
fn positioned_link_areas_use_final_terminal_coordinates() -> leptatui::app::Result<()> {
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
fn positioned_editor_cursor_uses_final_terminal_coordinates() -> leptatui::app::Result<()> {
    let positioned = input("A").with_focus(true).with_inline_style(
        fixture_size(6.0, 3.0)
            .position(Position::Absolute)
            .inset(cell_insets(Some(1.0), None, None, Some(3.0))),
    );
    let root = div((positioned,))
        .with_inline_style(fixture_size(10.0, 5.0).position(Position::Relative))
        .into_view();
    let mut terminal = render_view(root.as_view(), 10, 5)?;

    assert_eq!(rendered_lines(&terminal)[2].chars().nth(5), Some('A'));
    terminal.backend_mut().assert_cursor_position((6, 2));
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
fn wheel_targeting_uses_frontmost_global_paint_order() -> leptatui::app::Result<()> {
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
