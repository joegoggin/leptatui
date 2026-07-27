/// Verifies narrow row and column flex containers wrap with axis-specific gaps.
///
/// # Example Under Test
///
/// ```text
/// row: 5x6, children 2x1, gap(1, 1)
/// column: 6x5, children 1x2, gap(1, 1)
/// align-content: space-between
/// ```
///
/// # Assertions
///
/// - The narrow row retains two children on its first line and one on its second.
/// - The narrow column retains two children in its first column and one in its second.
/// - Item gaps separate siblings while cross-line distribution reaches both container edges.
#[test]
fn narrow_flex_axes_wrap_with_item_and_line_gaps() -> Result<()> {
    let row = div((
        fixed_flex_child("A", 2.0, 1.0),
        fixed_flex_child("B", 2.0, 1.0),
        fixed_flex_child("C", 2.0, 1.0),
    ))
    .with_inline_style(
        flex_fixture_size(5.0, 6.0)
            .display(Display::Flex)
            .flex_wrap(FlexWrap::Wrap)
            .gap(Axes::all(Length::cells(1.0)))
            .align_items(AlignItems::FlexStart)
            .align_content(AlignContent::SpaceBetween),
    )
    .into_view();
    let column = div((
        fixed_flex_child("A", 1.0, 2.0),
        fixed_flex_child("B", 1.0, 2.0),
        fixed_flex_child("C", 1.0, 2.0),
    ))
    .with_inline_style(
        flex_fixture_size(6.0, 5.0)
            .display(Display::Flex)
            .flex_direction(FlexDirection::Column)
            .flex_wrap(FlexWrap::Wrap)
            .gap(Axes::all(Length::cells(1.0)))
            .align_items(AlignItems::FlexStart)
            .align_content(AlignContent::SpaceBetween),
    )
    .into_view();

    let _row_terminal = render_layout_root(&row, 5, 6)?;
    let _column_terminal = render_layout_root(&column, 6, 5)?;

    assert_eq!(
        retained_flex_children(&row),
        [
            ratatui::layout::Rect::new(0, 0, 2, 1),
            ratatui::layout::Rect::new(3, 0, 2, 1),
            ratatui::layout::Rect::new(0, 5, 2, 1),
        ]
    );
    assert_eq!(
        retained_flex_children(&column),
        [
            ratatui::layout::Rect::new(0, 0, 1, 2),
            ratatui::layout::Rect::new(0, 3, 1, 2),
            ratatui::layout::Rect::new(5, 0, 1, 2),
        ]
    );
    Ok(())
}

/// Verifies reversed main and cross axes preserve source-order geometry.
///
/// # Example Under Test
///
/// ```text
/// 5x5 row-reverse flex container
/// children: A(2x1), B(2x1), C(2x1)
/// wrap: wrap-reverse
/// gap: 1 cell on both axes
/// ```
///
/// # Assertions
///
/// - Source children remain addressable as A, B, and C after visual reversal.
/// - A and B occupy the cross-end line in reversed main-axis order.
/// - C occupies the preceding reverse-wrapped line without exceeding the content box.
#[test]
fn reverse_direction_and_wrap_reverse_keep_stable_source_rectangles() -> Result<()> {
    let root = div((
        fixed_flex_child("A", 2.0, 1.0),
        fixed_flex_child("B", 2.0, 1.0),
        fixed_flex_child("C", 2.0, 1.0),
    ))
    .with_inline_style(
        flex_fixture_size(5.0, 5.0)
            .display(Display::Flex)
            .flex_direction(FlexDirection::RowReverse)
            .flex_wrap(FlexWrap::WrapReverse)
            .gap(Axes::all(Length::cells(1.0)))
            .align_items(AlignItems::FlexStart)
            .align_content(AlignContent::FlexStart),
    )
    .into_view();

    let _terminal = render_layout_root(&root, 5, 5)?;
    let children = retained_flex_children(&root);

    assert_eq!(
        children,
        [
            ratatui::layout::Rect::new(3, 4, 2, 1),
            ratatui::layout::Rect::new(0, 4, 2, 1),
            ratatui::layout::Rect::new(3, 2, 2, 1),
        ]
    );
    assert!(
        children
            .iter()
            .all(|child| child.right() <= 5 && child.bottom() <= 5)
    );
    Ok(())
}

/// Verifies odd flex remainders round cumulatively within the parent.
///
/// # Example Under Test
///
/// ```text
/// 10x1 row flex container
/// three children with flex-basis 0 and flex-grow 1
/// ```
///
/// # Assertions
///
/// - The three rounded widths are three, four, and three cells.
/// - Each child starts at the preceding child's rounded end.
/// - The final child ends exactly at the parent content-box edge.
///
/// # Why
///
/// Independently rounding three equal fractional widths can exceed or underfill
/// the ten-cell parent, so terminal geometry must retain cumulative rounding.
#[test]
fn odd_flex_remainders_round_cumulatively_without_overflow() -> Result<()> {
    let child_style = TuiStyle::new()
        .flex_basis(Dimension::from(Length::cells(0.0)))
        .flex_grow(1.0)
        .size(LayoutSize::new(
            Dimension::Auto,
            Dimension::from(Length::cells(1.0)),
        ));
    let root = div((
        text("A").with_inline_style(child_style.clone()),
        text("B").with_inline_style(child_style.clone()),
        text("C").with_inline_style(child_style),
    ))
    .with_inline_style(
        flex_fixture_size(10.0, 1.0)
            .display(Display::Flex)
            .align_items(AlignItems::FlexStart),
    )
    .into_view();

    let _terminal = render_layout_root(&root, 10, 1)?;
    let children = retained_flex_children(&root);
    let content_box = root
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("root geometry")
        .content_box;

    assert_eq!(
        children,
        [
            ratatui::layout::Rect::new(0, 0, 3, 1),
            ratatui::layout::Rect::new(3, 0, 4, 1),
            ratatui::layout::Rect::new(7, 0, 3, 1),
        ]
    );
    assert_eq!(children[0].right(), children[1].x);
    assert_eq!(children[1].right(), children[2].x);
    assert_eq!(children[2].right(), content_box.right());
    Ok(())
}

/// Verifies zero-sized reversed wrapping retains saturated terminal rectangles.
///
/// # Example Under Test
///
/// ```text
/// 0x0 row-reverse flex container
/// two 0x0 children
/// wrap: wrap-reverse
/// gap: 1 cell on both axes
/// ```
///
/// # Assertions
///
/// - The parent retains a true zero-sized content box.
/// - Both children retain zero width and height.
/// - The reverse-wrapped line gap places the first zero-area child one row later.
/// - Both child rectangles remain saturated within the one-cell render viewport.
#[test]
fn zero_sized_reverse_wrapping_saturates_terminal_geometry() -> Result<()> {
    let root = div((
        fixed_flex_child("A", 0.0, 0.0),
        fixed_flex_child("B", 0.0, 0.0),
    ))
    .with_inline_style(
        flex_fixture_size(0.0, 0.0)
            .display(Display::Flex)
            .flex_direction(FlexDirection::RowReverse)
            .flex_wrap(FlexWrap::WrapReverse)
            .gap(Axes::all(Length::cells(1.0)))
            .align_items(AlignItems::FlexStart),
    )
    .into_view();

    let _terminal = render_layout_root(&root, 1, 1)?;
    let children = retained_flex_children(&root);
    let content_box = root
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("root geometry")
        .content_box;

    assert_eq!(content_box, ratatui::layout::Rect::new(0, 0, 0, 0));
    assert_eq!(
        children,
        [
            ratatui::layout::Rect::new(0, 1, 0, 0),
            ratatui::layout::Rect::new(0, 0, 0, 0),
        ]
    );
    assert!(
        children
            .iter()
            .all(|child| child.right() <= 1 && child.bottom() <= 1)
    );
    Ok(())
}

/// Verifies multiline flex geometry paints a stable terminal snapshot.
///
/// # Example Under Test
///
/// ```text
/// 5x3 wrapped row with labels AA, BB, and CC
/// child size: 2x1
/// gap: 1 cell on both axes
/// ```
///
/// # Assertions
///
/// - The first row paints `AA`, one gap cell, and `BB`.
/// - The line gap leaves the middle terminal row blank.
/// - The wrapped `CC` label paints at the start of the final row.
#[test]
fn wrapped_flex_lines_paint_a_stable_terminal_snapshot() -> Result<()> {
    let root = div((
        fixed_flex_child("AA", 2.0, 1.0),
        fixed_flex_child("BB", 2.0, 1.0),
        fixed_flex_child("CC", 2.0, 1.0),
    ))
    .with_inline_style(
        flex_fixture_size(5.0, 3.0)
            .display(Display::Flex)
            .flex_wrap(FlexWrap::Wrap)
            .gap(Axes::all(Length::cells(1.0)))
            .align_items(AlignItems::FlexStart)
            .align_content(AlignContent::FlexStart),
    )
    .into_view();

    let terminal = render_layout_root(&root, 5, 3)?;

    assert_eq!(rendered_text(&terminal), "AA BB     CC   ");
    assert_eq!(
        retained_flex_children(&root),
        [
            ratatui::layout::Rect::new(0, 0, 2, 1),
            ratatui::layout::Rect::new(3, 0, 2, 1),
            ratatui::layout::Rect::new(0, 2, 2, 1),
        ]
    );
    Ok(())
}
