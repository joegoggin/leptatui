// Margin, padding, border geometry, and painting integration tests.

/// Verifies asymmetric margins position normal-flow boxes and collapse between siblings.
///
/// # Example Under Test
///
/// ```text
/// 24x18 bordered parent
/// first child: auto width, 8 rows, margins 1 2 3 4
/// second child: 1 row, margins 2 0 0 1
/// ```
///
/// # Assertions
///
/// - The first child starts after the parent's content origin and its top and left margins.
/// - The first child's automatic width excludes both horizontal margins.
/// - The three-cell bottom margin and two-cell following top margin collapse to three cells.
/// - The following sibling retains its independent left margin.
#[test]
fn asymmetric_margins_position_and_separate_block_siblings() -> Result<()> {
    let first = text("A").with_inline_style(
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::Auto,
                Dimension::from(Length::cells(8.0)),
            ))
            .margin(Edges::new(
                Length::cells(1.0).into(),
                Length::cells(2.0).into(),
                Length::cells(3.0).into(),
                Length::cells(4.0).into(),
            )),
    );
    let second = text("B").with_inline_style(
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::Auto,
                Dimension::from(Length::cells(1.0)),
            ))
            .margin(Edges::new(
                Length::cells(2.0).into(),
                Length::cells(0.0).into(),
                Length::cells(0.0).into(),
                Length::cells(1.0).into(),
            )),
    );
    let root = div((first, second))
        .with_inline_style(
            TuiStyle::new()
                .borders(Borders::ALL)
                .box_sizing(BoxSizing::BorderBox)
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(24.0)),
                    Dimension::from(Length::cells(18.0)),
                )),
        )
        .into_view();

    let _terminal = render_layout_root(&root, 24, 18)?;
    let children = root
        .downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views();

    assert_eq!(
        retained_border_box(&children[0]),
        ratatui::layout::Rect::new(5, 2, 16, 8)
    );
    assert_eq!(
        retained_border_box(&children[1]),
        ratatui::layout::Rect::new(2, 13, 21, 1)
    );
    Ok(())
}

/// Verifies asymmetric padding and enabled border sides define every retained box.
///
/// # Example Under Test
///
/// ```text
/// border box: 10x8
/// borders: left, top, bottom
/// padding: left 1, right 2, top 2, bottom 1
/// ```
///
/// # Assertions
///
/// - Enabled border sides inset the padding box by exactly one cell.
/// - The disabled right border consumes no layout width.
/// - Each authored padding edge independently insets the content box.
/// - The content renders at the retained content-box origin.
#[test]
fn asymmetric_padding_and_border_sides_define_retained_boxes() -> Result<()> {
    let root = block(text("X"))
        .with_inline_style(
            TuiStyle::new()
                .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
                .box_sizing(BoxSizing::BorderBox)
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(10.0)),
                    Dimension::from(Length::cells(8.0)),
                ))
                .padding(TuiSpacing::new(1, 2, 2, 1)),
        )
        .into_view();

    let terminal = render_layout_root(&root, 12, 10)?;
    let geometry = root
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("block geometry");

    assert_eq!(
        geometry.border_box,
        ratatui::layout::Rect::new(0, 0, 10, 8)
    );
    assert_eq!(
        geometry.padding_box,
        ratatui::layout::Rect::new(1, 1, 9, 6)
    );
    assert_eq!(
        geometry.content_box,
        ratatui::layout::Rect::new(2, 3, 6, 3)
    );
    assert_eq!(symbol_position(&terminal, "X", 12), (2, 3));
    Ok(())
}

/// Verifies margins stay unpainted while backgrounds and rounded borders use the border box.
///
/// # Example Under Test
///
/// ```text
/// child margin: 1 cell
/// child border box: 8x5
/// border type: rounded
/// background: blue
/// padding: 1 cell
/// ```
///
/// # Assertions
///
/// - The child border box begins after its margins.
/// - Rounded corner glyphs occupy the border-box corners.
/// - Border, padding, content, and trailing cells receive the authored background.
/// - Cells in the surrounding margin retain the terminal's default background.
#[test]
fn background_and_rounded_borders_paint_only_the_border_box() -> Result<()> {
    let child = block(text("X")).with_inline_style(
        TuiStyle::new()
            .background(Color::Blue)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(8.0)),
                Dimension::from(Length::cells(5.0)),
            ))
            .margin(Edges::all(Length::cells(1.0).into()))
            .padding(TuiSpacing::uniform(1)),
    );
    let root = div((child,))
        .with_inline_style(
            TuiStyle::new()
                .box_sizing(BoxSizing::BorderBox)
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(12.0)),
                    Dimension::from(Length::cells(8.0)),
                )),
        )
        .into_view();

    let terminal = render_layout_root(&root, 12, 8)?;
    let child = &root
        .downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views()[0];
    let geometry = child
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("child geometry");

    assert_eq!(
        geometry.border_box,
        ratatui::layout::Rect::new(1, 1, 8, 5)
    );
    assert_eq!(cell_symbol(&terminal, 1, 1, 12), symbol_border::ROUNDED.top_left);
    assert_eq!(
        cell_symbol(&terminal, 8, 5, 12),
        symbol_border::ROUNDED.bottom_right
    );
    for (x, y) in [(1, 1), (2, 2), (3, 3), (7, 3)] {
        assert_eq!(cell_colors(&terminal, x, y, 12).1, Color::Blue);
    }
    assert_eq!(symbol_position(&terminal, "X", 12), (3, 3));
    assert_eq!(cell_colors(&terminal, 0, 1, 12).1, Color::Reset);
    assert_eq!(cell_colors(&terminal, 9, 1, 12).1, Color::Reset);
    Ok(())
}

/// Verifies leaf painting consumes retained border and content rectangles.
///
/// # Example Under Test
///
/// ```text
/// 8x5 text("X")
/// blue background, borders: all, padding: 1
/// ```
///
/// # Assertions
///
/// - Border glyphs occupy the retained border-box corners.
/// - The text begins at the retained content-box origin.
/// - Border, padding, and content cells share the authored background.
#[test]
fn styled_leaf_paints_from_retained_box_geometry() -> Result<()> {
    let root = div((text("X").with_inline_style(
        TuiStyle::new()
            .background(Color::Blue)
            .borders(Borders::ALL)
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(8.0)),
                Dimension::from(Length::cells(5.0)),
            ))
            .padding(TuiSpacing::uniform(1)),
    ),))
    .into_view();

    let terminal = render_layout_root(&root, 10, 6)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 10), symbol_border::PLAIN.top_left);
    assert_eq!(
        cell_symbol(&terminal, 7, 4, 10),
        symbol_border::PLAIN.bottom_right
    );
    assert_eq!(symbol_position(&terminal, "X", 10), (2, 2));
    for (x, y) in [(0, 0), (1, 1), (2, 2), (6, 3)] {
        assert_eq!(cell_colors(&terminal, x, y, 10).1, Color::Blue);
    }
    Ok(())
}

/// Verifies zero and chrome-constrained boxes retain safe saturated geometry.
///
/// # Example Under Test
///
/// ```text
/// child 1: authored 0x0 text box
/// child 2: authored 0x0 bordered block with one-cell padding
/// child 3: one-row text box
/// ```
///
/// # Assertions
///
/// - The unadorned empty child retains a true zero-sized border box.
/// - Borders and padding expand the second border box only to its required chrome.
/// - The second content box saturates to zero width and height without underflow.
/// - The following sibling renders immediately after the chrome-constrained box.
#[test]
fn zero_sized_boxes_saturate_inner_geometry_without_affecting_following_paint() -> Result<()> {
    let zero = text("").with_inline_style(
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::all(Dimension::from(Length::cells(0.0)))),
    );
    let chrome = block(text(""))
        .with_inline_style(
            TuiStyle::new()
                .borders(Borders::ALL)
                .box_sizing(BoxSizing::BorderBox)
                .size(LayoutSize::all(Dimension::from(Length::cells(0.0))))
                .padding(TuiSpacing::uniform(1)),
        );
    let root = div((zero, chrome, text("A"))).into_view();

    let terminal = render_layout_root(&root, 8, 8)?;
    let children = root
        .downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views();
    let zero_geometry = children[0]
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("zero geometry");
    let chrome_geometry = children[1]
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("chrome geometry");

    assert_eq!(
        zero_geometry.border_box,
        ratatui::layout::Rect::new(0, 0, 0, 0)
    );
    assert_eq!(
        chrome_geometry.border_box,
        ratatui::layout::Rect::new(0, 0, 4, 4)
    );
    assert_eq!(
        chrome_geometry.padding_box,
        ratatui::layout::Rect::new(1, 1, 2, 2)
    );
    assert_eq!(
        chrome_geometry.content_box,
        ratatui::layout::Rect::new(2, 2, 0, 0)
    );
    assert_eq!(symbol_position(&terminal, "A", 8), (0, 4));
    Ok(())
}
