/// Returns a definite border-box style for one flex fixture.
///
/// # Arguments
///
/// * `width` — Fixture width in terminal cells.
/// * `height` — Fixture height in terminal cells.
///
/// # Returns
///
/// A [`TuiStyle`] containing the requested border-box dimensions.
fn flex_fixture_size(width: f32, height: f32) -> TuiStyle {
    TuiStyle::new()
        .box_sizing(BoxSizing::BorderBox)
        .size(LayoutSize::new(
            Dimension::from(Length::cells(width)),
            Dimension::from(Length::cells(height)),
        ))
}

/// Returns one fixed-size, non-shrinking flex child.
///
/// # Arguments
///
/// * `label` — Text painted by the child.
/// * `width` — Child width in terminal cells.
/// * `height` — Child height in terminal cells.
///
/// # Returns
///
/// An [`AnyView`] containing the styled text child.
fn fixed_flex_child(label: &'static str, width: f32, height: f32) -> AnyView {
    text(label)
        .with_inline_style(flex_fixture_size(width, height).flex_shrink(0.0))
        .into_view()
}

/// Returns every retained child border box from a division root.
///
/// # Arguments
///
/// * `root` — Erased division view containing retained child geometry.
///
/// # Returns
///
/// A [`Vec`] containing child rectangles in source order.
fn retained_flex_children(root: &AnyView) -> Vec<ratatui::layout::Rect> {
    root.downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views()
        .iter()
        .map(retained_border_box)
        .collect()
}

/// Verifies every flex direction controls the main axis and source-order origin.
///
/// # Example Under Test
///
/// ```text
/// 8x6 flex container
/// children: A(2x1), B(2x1)
/// directions: row, row-reverse, column, column-reverse
/// ```
///
/// # Assertions
///
/// - Row directions place children horizontally from the selected edge.
/// - Column directions place children vertically from the selected edge.
/// - Reverse directions preserve source order while reversing placement.
#[test]
fn flex_directions_place_children_on_the_selected_main_axis() -> leptatui::app::Result<()> {
    let cases = [
        (
            FlexDirection::Row,
            [
                ratatui::layout::Rect::new(0, 0, 2, 1),
                ratatui::layout::Rect::new(2, 0, 2, 1),
            ],
        ),
        (
            FlexDirection::RowReverse,
            [
                ratatui::layout::Rect::new(6, 0, 2, 1),
                ratatui::layout::Rect::new(4, 0, 2, 1),
            ],
        ),
        (
            FlexDirection::Column,
            [
                ratatui::layout::Rect::new(0, 0, 2, 1),
                ratatui::layout::Rect::new(0, 1, 2, 1),
            ],
        ),
        (
            FlexDirection::ColumnReverse,
            [
                ratatui::layout::Rect::new(0, 5, 2, 1),
                ratatui::layout::Rect::new(0, 4, 2, 1),
            ],
        ),
    ];

    for (direction, expected) in cases {
        let root = div((
            fixed_flex_child("A", 2.0, 1.0),
            fixed_flex_child("B", 2.0, 1.0),
        ))
        .with_inline_style(
            flex_fixture_size(8.0, 6.0)
                .display(Display::Flex)
                .flex_direction(direction)
                .align_items(AlignItems::FlexStart),
        )
        .into_view();

        let _terminal = render_layout_root(&root, 8, 6)?;

        assert_eq!(
            retained_flex_children(&root),
            expected,
            "direction: {direction:?}"
        );
    }
    Ok(())
}

/// Verifies every flex wrapping mode controls line creation and cross-axis order.
///
/// # Example Under Test
///
/// ```text
/// 5x6 row flex container
/// children: A(2x1), B(2x1), C(2x1)
/// wrap modes: nowrap, wrap, wrap-reverse
/// ```
///
/// # Assertions
///
/// - No-wrap keeps all children on one line even when they overflow.
/// - Wrap moves the third child to the next cross-axis line.
/// - Wrap-reverse reverses the two flex-line positions.
#[test]
fn flex_wrap_modes_control_line_creation_and_cross_axis_order() -> leptatui::app::Result<()> {
    let cases = [
        (
            FlexWrap::NoWrap,
            [
                ratatui::layout::Rect::new(0, 0, 2, 1),
                ratatui::layout::Rect::new(2, 0, 2, 1),
                ratatui::layout::Rect::new(4, 0, 2, 1),
            ],
        ),
        (
            FlexWrap::Wrap,
            [
                ratatui::layout::Rect::new(0, 0, 2, 1),
                ratatui::layout::Rect::new(2, 0, 2, 1),
                ratatui::layout::Rect::new(0, 3, 2, 1),
            ],
        ),
        (
            FlexWrap::WrapReverse,
            [
                ratatui::layout::Rect::new(0, 5, 2, 1),
                ratatui::layout::Rect::new(2, 5, 2, 1),
                ratatui::layout::Rect::new(0, 2, 2, 1),
            ],
        ),
    ];

    for (wrap, expected) in cases {
        let root = div((
            fixed_flex_child("A", 2.0, 1.0),
            fixed_flex_child("B", 2.0, 1.0),
            fixed_flex_child("C", 2.0, 1.0),
        ))
        .with_inline_style(
            flex_fixture_size(5.0, 6.0)
                .display(Display::Flex)
                .flex_wrap(wrap)
                .align_items(AlignItems::FlexStart),
        )
        .into_view();

        let _terminal = render_layout_root(&root, 5, 6)?;

        assert_eq!(
            retained_flex_children(&root),
            expected,
            "wrap: {wrap:?}"
        );
    }
    Ok(())
}

/// Verifies horizontal and vertical gaps separate items and flex lines.
///
/// # Example Under Test
///
/// ```text
/// row: gap(2, 0)
/// column: gap(0, 2)
/// wrapped row: gap(1, 2)
/// ```
///
/// # Assertions
///
/// - A horizontal gap separates row children by two cells.
/// - A vertical gap separates column children by two cells.
/// - Wrapped rows apply the horizontal item gap and vertical line gap.
#[test]
fn flex_gaps_apply_to_items_and_wrapped_lines() -> leptatui::app::Result<()> {
    let row = div((
        fixed_flex_child("A", 2.0, 1.0),
        fixed_flex_child("B", 2.0, 1.0),
    ))
    .with_inline_style(
        flex_fixture_size(10.0, 4.0)
            .display(Display::Flex)
            .gap(Axes::new(Length::cells(2.0), Length::cells(0.0)))
            .align_items(AlignItems::FlexStart),
    )
    .into_view();
    let column = div((
        fixed_flex_child("A", 2.0, 1.0),
        fixed_flex_child("B", 2.0, 1.0),
    ))
    .with_inline_style(
        flex_fixture_size(10.0, 6.0)
            .display(Display::Flex)
            .flex_direction(FlexDirection::Column)
            .gap(Axes::new(Length::cells(0.0), Length::cells(2.0)))
            .align_items(AlignItems::FlexStart),
    )
    .into_view();
    let wrapped = div((
        fixed_flex_child("A", 2.0, 1.0),
        fixed_flex_child("B", 2.0, 1.0),
        fixed_flex_child("C", 2.0, 1.0),
    ))
    .with_inline_style(
        flex_fixture_size(5.0, 6.0)
            .display(Display::Flex)
            .flex_wrap(FlexWrap::Wrap)
            .gap(Axes::new(Length::cells(1.0), Length::cells(2.0)))
            .align_items(AlignItems::FlexStart)
            .align_content(AlignContent::FlexStart),
    )
    .into_view();

    let _row_terminal = render_layout_root(&row, 10, 4)?;
    let _column_terminal = render_layout_root(&column, 10, 6)?;
    let _wrapped_terminal = render_layout_root(&wrapped, 5, 6)?;

    assert_eq!(
        retained_flex_children(&row),
        [
            ratatui::layout::Rect::new(0, 0, 2, 1),
            ratatui::layout::Rect::new(4, 0, 2, 1),
        ]
    );
    assert_eq!(
        retained_flex_children(&column),
        [
            ratatui::layout::Rect::new(0, 0, 2, 1),
            ratatui::layout::Rect::new(0, 3, 2, 1),
        ]
    );
    assert_eq!(
        retained_flex_children(&wrapped),
        [
            ratatui::layout::Rect::new(0, 0, 2, 1),
            ratatui::layout::Rect::new(3, 0, 2, 1),
            ratatui::layout::Rect::new(0, 3, 2, 1),
        ]
    );
    Ok(())
}

/// Verifies every justify-content value distributes children on the main axis.
///
/// # Example Under Test
///
/// ```text
/// 10x2 row flex container
/// children: A(2x1), B(2x1)
/// justify-content: every public variant
/// ```
///
/// # Assertions
///
/// - Start, end, center, flex-relative, and stretch values pack as specified.
/// - Space-between, space-around, and space-evenly distribute six free cells.
#[test]
fn justify_content_variants_distribute_main_axis_space() -> leptatui::app::Result<()> {
    let cases = [
        (JustifyContent::Start, [0, 2]),
        (JustifyContent::End, [6, 8]),
        (JustifyContent::FlexStart, [0, 2]),
        (JustifyContent::FlexEnd, [6, 8]),
        (JustifyContent::Center, [3, 5]),
        (JustifyContent::Stretch, [0, 2]),
        (JustifyContent::SpaceBetween, [0, 8]),
        (JustifyContent::SpaceAround, [2, 7]),
        (JustifyContent::SpaceEvenly, [2, 6]),
    ];

    for (justify, expected_x) in cases {
        let root = div((
            fixed_flex_child("A", 2.0, 1.0),
            fixed_flex_child("B", 2.0, 1.0),
        ))
        .with_inline_style(
            flex_fixture_size(10.0, 2.0)
                .display(Display::Flex)
                .justify_content(justify)
                .align_items(AlignItems::FlexStart),
        )
        .into_view();

        let _terminal = render_layout_root(&root, 10, 2)?;
        let children = retained_flex_children(&root);

        assert_eq!(
            [children[0].x, children[1].x],
            expected_x,
            "justify-content: {justify:?}"
        );
    }
    Ok(())
}

/// Verifies every align-items value positions children on the cross axis.
///
/// # Example Under Test
///
/// ```text
/// 6x6 row flex container
/// children: A(2x1), B(2x2)
/// align-items: every public variant
/// ```
///
/// # Assertions
///
/// - Start, end, flex-relative, center, and baseline values position both children.
/// - Stretch expands an auto-height child to the six-cell cross size.
#[test]
fn align_items_variants_position_children_on_the_cross_axis() -> leptatui::app::Result<()> {
    let cases = [
        (AlignItems::Start, [0, 0]),
        (AlignItems::End, [5, 4]),
        (AlignItems::FlexStart, [0, 0]),
        (AlignItems::FlexEnd, [5, 4]),
        (AlignItems::Center, [3, 2]),
        (AlignItems::Baseline, [1, 0]),
    ];

    for (align, expected_y) in cases {
        let root = div((
            fixed_flex_child("A", 2.0, 1.0),
            fixed_flex_child("B", 2.0, 2.0),
        ))
        .with_inline_style(
            flex_fixture_size(6.0, 6.0)
                .display(Display::Flex)
                .align_items(align),
        )
        .into_view();

        let _terminal = render_layout_root(&root, 6, 6)?;
        let children = retained_flex_children(&root);

        assert_eq!(
            [children[0].y, children[1].y],
            expected_y,
            "align-items: {align:?}"
        );
    }

    let stretched = div((text("A").with_inline_style(
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(2.0)),
                Dimension::Auto,
            )),
    ),))
    .with_inline_style(
        flex_fixture_size(6.0, 6.0)
            .display(Display::Flex)
            .align_items(AlignItems::Stretch),
    )
    .into_view();

    let _terminal = render_layout_root(&stretched, 6, 6)?;
    assert_eq!(
        retained_flex_children(&stretched),
        [ratatui::layout::Rect::new(0, 0, 2, 6)]
    );
    Ok(())
}

/// Verifies every align-content value distributes wrapped flex lines.
///
/// # Example Under Test
///
/// ```text
/// 5x8 wrapped row flex container
/// children: A(3x1), B(3x1)
/// align-content: every public variant
/// ```
///
/// # Assertions
///
/// - Start, end, flex-relative, and center values pack both lines.
/// - Stretch expands each line's cross size without changing fixed child height.
/// - Space distribution values place both lines across six free cells.
#[test]
fn align_content_variants_distribute_wrapped_lines() -> leptatui::app::Result<()> {
    let cases = [
        (AlignContent::Start, [0, 1]),
        (AlignContent::End, [6, 7]),
        (AlignContent::FlexStart, [0, 1]),
        (AlignContent::FlexEnd, [6, 7]),
        (AlignContent::Center, [3, 4]),
        (AlignContent::Stretch, [0, 4]),
        (AlignContent::SpaceBetween, [0, 7]),
        (AlignContent::SpaceAround, [2, 6]),
        (AlignContent::SpaceEvenly, [2, 5]),
    ];

    for (align, expected_y) in cases {
        let root = div((
            fixed_flex_child("A", 3.0, 1.0),
            fixed_flex_child("B", 3.0, 1.0),
        ))
        .with_inline_style(
            flex_fixture_size(5.0, 8.0)
                .display(Display::Flex)
                .flex_wrap(FlexWrap::Wrap)
                .align_items(AlignItems::FlexStart)
                .align_content(align),
        )
        .into_view();

        let _terminal = render_layout_root(&root, 5, 8)?;
        let children = retained_flex_children(&root);

        assert_eq!(
            [children[0].y, children[1].y],
            expected_y,
            "align-content: {align:?}"
        );
    }
    Ok(())
}

/// Verifies flex geometry is rebuilt when the terminal width changes.
///
/// # Example Under Test
///
/// ```text
/// 100%-wide wrapped row with three 3x1 children and gap(1, 1)
/// viewport widths: 11, then 7, then 11
/// ```
///
/// # Assertions
///
/// - The wide viewport places all three children on one line.
/// - The narrow viewport moves the third child onto a second line.
/// - Returning to the wide viewport replaces the retained wrapped geometry.
#[test]
fn flex_container_rebuilds_wrapped_geometry_after_terminal_resize() -> leptatui::app::Result<()> {
    let root = div((
        fixed_flex_child("A", 3.0, 1.0),
        fixed_flex_child("B", 3.0, 1.0),
        fixed_flex_child("C", 3.0, 1.0),
    ))
    .with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::cells(4.0)),
            ))
            .flex_wrap(FlexWrap::Wrap)
            .gap(Axes::all(Length::cells(1.0)))
            .align_items(AlignItems::FlexStart)
            .align_content(AlignContent::FlexStart),
    )
    .into_view();

    let _wide = render_layout_root(&root, 11, 4)?;
    assert_eq!(
        retained_flex_children(&root),
        [
            ratatui::layout::Rect::new(0, 0, 3, 1),
            ratatui::layout::Rect::new(4, 0, 3, 1),
            ratatui::layout::Rect::new(8, 0, 3, 1),
        ]
    );

    let _narrow = render_layout_root(&root, 7, 4)?;
    assert_eq!(
        retained_flex_children(&root),
        [
            ratatui::layout::Rect::new(0, 0, 3, 1),
            ratatui::layout::Rect::new(4, 0, 3, 1),
            ratatui::layout::Rect::new(0, 2, 3, 1),
        ]
    );

    let _wide_again = render_layout_root(&root, 11, 4)?;
    assert_eq!(
        retained_flex_children(&root),
        [
            ratatui::layout::Rect::new(0, 0, 3, 1),
            ratatui::layout::Rect::new(4, 0, 3, 1),
            ratatui::layout::Rect::new(8, 0, 3, 1),
        ]
    );
    Ok(())
}
