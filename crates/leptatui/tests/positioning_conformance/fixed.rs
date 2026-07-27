//! Fixed viewport positioning conformance tests.

use super::*;

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
