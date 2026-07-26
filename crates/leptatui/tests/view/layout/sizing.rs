/// Returns the retained border box for one view after layout.
///
/// # Arguments
///
/// * `view` — View whose retained layout geometry should be inspected.
///
/// # Returns
///
/// A [`ratatui::layout::Rect`] containing the view's border box.
fn retained_border_box(view: &AnyView) -> ratatui::layout::Rect {
    view.style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("retained layout geometry")
        .border_box
}

/// Returns one retained child border box from a [`leptatui::DivView`].
///
/// # Arguments
///
/// * `root` — Erased division view containing the requested child.
/// * `index` — Child index whose geometry should be inspected.
///
/// # Returns
///
/// A [`ratatui::layout::Rect`] containing the child's border box.
fn retained_div_child_border_box(root: &AnyView, index: usize) -> ratatui::layout::Rect {
    let children = root
        .downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views();
    retained_border_box(&children[index])
}

/// Verifies preferred, minimum, and maximum sizes resolve in terminal cells.
///
/// # Example Under Test
///
/// ```text
/// preferred width: 4, 10, or 8 cells
/// minimum width: auto or 6 cells
/// maximum width: auto or 6 cells
/// preferred height: 2 cells
/// ```
///
/// # Assertions
///
/// - An unconstrained preferred size is retained.
/// - A minimum width raises a smaller preferred width.
/// - A maximum width lowers a larger preferred width.
#[test]
fn preferred_minimum_and_maximum_sizes_resolve_from_table() -> Result<()> {
    let cases = [
        (
            "preferred",
            Dimension::Auto,
            Dimension::Auto,
            8.0,
            ratatui::layout::Rect::new(0, 0, 8, 2),
        ),
        (
            "minimum",
            Dimension::from(Length::cells(6.0)),
            Dimension::Auto,
            4.0,
            ratatui::layout::Rect::new(0, 0, 6, 2),
        ),
        (
            "maximum",
            Dimension::Auto,
            Dimension::from(Length::cells(6.0)),
            10.0,
            ratatui::layout::Rect::new(0, 0, 6, 2),
        ),
    ];

    for (name, min_width, max_width, preferred_width, expected) in cases {
        let root = text(name)
            .with_inline_style(
                TuiStyle::new()
                    .box_sizing(BoxSizing::BorderBox)
                    .size(LayoutSize::new(
                        Dimension::from(Length::cells(preferred_width)),
                        Dimension::from(Length::cells(2.0)),
                    ))
                    .min_size(LayoutSize::new(min_width, Dimension::Auto))
                    .max_size(LayoutSize::new(max_width, Dimension::Auto)),
            )
            .into_view();

        let _terminal = render_layout_root(&root, 20, 6)?;

        assert_eq!(retained_border_box(&root), expected, "case: {name}");
    }
    Ok(())
}

/// Verifies a preferred aspect ratio derives either automatic axis.
///
/// # Example Under Test
///
/// ```text
/// width: 8, height: auto, aspect_ratio: 2
/// width: auto, height: 4, aspect_ratio: 2
/// ```
///
/// # Assertions
///
/// - A definite width derives a four-cell automatic height.
/// - A definite height derives an eight-cell automatic width.
#[test]
fn aspect_ratio_derives_automatic_width_or_height() -> Result<()> {
    let width_driven = text("width")
        .with_inline_style(
            TuiStyle::new()
                .box_sizing(BoxSizing::BorderBox)
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(8.0)),
                    Dimension::Auto,
                ))
                .aspect_ratio(2.0),
        )
        .into_view();
    let height_driven_child = text("height").with_inline_style(
        TuiStyle::new()
            .position(Position::Absolute)
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::Auto,
                Dimension::from(Length::cells(4.0)),
            ))
            .aspect_ratio(2.0),
    );
    let height_driven = div((height_driven_child,))
        .with_inline_style(
            TuiStyle::new()
                .position(Position::Relative)
                .box_sizing(BoxSizing::BorderBox)
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(20.0)),
                    Dimension::from(Length::cells(8.0)),
                )),
        )
        .into_view();

    let _width_terminal = render_layout_root(&width_driven, 20, 8)?;
    let _height_terminal = render_layout_root(&height_driven, 20, 8)?;

    assert_eq!(
        retained_border_box(&width_driven),
        ratatui::layout::Rect::new(0, 0, 8, 4)
    );
    assert_eq!(
        retained_div_child_border_box(&height_driven, 0),
        ratatui::layout::Rect::new(0, 0, 8, 4)
    );
    Ok(())
}

/// Verifies invalid aspect ratios preserve automatic sizing.
///
/// # Example Under Test
///
/// ```text
/// width: 8, height: auto
/// aspect_ratio: 0, -1, NaN, or infinity
/// text content height: 1
/// ```
///
/// # Assertions
///
/// - Each invalid ratio is ignored.
/// - Intrinsic text measurement supplies the automatic one-cell height.
#[test]
fn invalid_aspect_ratios_fall_back_to_automatic_sizing() -> Result<()> {
    for ratio in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let root = text("auto")
            .with_inline_style(
                TuiStyle::new()
                    .box_sizing(BoxSizing::BorderBox)
                    .size(LayoutSize::new(
                        Dimension::from(Length::cells(8.0)),
                        Dimension::Auto,
                    ))
                    .aspect_ratio(ratio),
            )
            .into_view();

        let _terminal = render_layout_root(&root, 20, 4)?;

        assert_eq!(
            retained_border_box(&root),
            ratatui::layout::Rect::new(0, 0, 8, 1),
            "ratio: {ratio:?}"
        );
    }
    Ok(())
}

/// Verifies percentage sizes and constraints use the nested containing block.
///
/// # Example Under Test
///
/// ```text
/// containing block width: 20
/// preferred child widths: 50%, 25%, and 75%
/// minimum child width: auto, 8, and auto
/// maximum child width: auto, auto, and 12
/// ```
///
/// # Assertions
///
/// - An unconstrained percentage resolves against the parent width.
/// - A minimum size clamps a smaller resolved percentage.
/// - A maximum size clamps a larger resolved percentage.
#[test]
fn nested_percentage_sizes_and_constraints_resolve_from_table() -> Result<()> {
    let cases = [
        ("percentage", 50.0, Dimension::Auto, Dimension::Auto, 10),
        (
            "minimum",
            25.0,
            Dimension::from(Length::cells(8.0)),
            Dimension::Auto,
            8,
        ),
        (
            "maximum",
            75.0,
            Dimension::Auto,
            Dimension::from(Length::cells(12.0)),
            12,
        ),
    ];

    for (name, percent, min_width, max_width, expected_width) in cases {
        let child = text(name).with_inline_style(
            TuiStyle::new()
                .box_sizing(BoxSizing::BorderBox)
                .size(LayoutSize::new(
                    Dimension::from(Length::percent(percent)),
                    Dimension::from(Length::cells(2.0)),
                ))
                .min_size(LayoutSize::new(min_width, Dimension::Auto))
                .max_size(LayoutSize::new(max_width, Dimension::Auto)),
        );
        let root = div((child,))
            .with_inline_style(
                TuiStyle::new()
                    .box_sizing(BoxSizing::BorderBox)
                    .size(LayoutSize::new(
                        Dimension::from(Length::cells(20.0)),
                        Dimension::from(Length::cells(6.0)),
                    )),
            )
            .into_view();

        let _terminal = render_layout_root(&root, 24, 8)?;

        assert_eq!(
            retained_div_child_border_box(&root, 0),
            ratatui::layout::Rect::new(0, 0, expected_width, 2),
            "case: {name}"
        );
    }
    Ok(())
}

/// Verifies every viewport unit is recomputed from the current terminal size.
///
/// # Example Under Test
///
/// ```text
/// size: 50vw x 25vh, viewport: 20x12
/// size: 50vmin x 25vmax, viewport: 12x24
/// ```
///
/// # Assertions
///
/// - Width-relative and height-relative units use their matching viewport axes.
/// - Minimum-axis and maximum-axis units use the current smaller and larger axes.
#[test]
fn viewport_relative_sizes_resolve_from_table() -> Result<()> {
    let cases = [
        (
            "width-height",
            Length::vw(50.0),
            Length::vh(25.0),
            20,
            12,
            ratatui::layout::Rect::new(0, 0, 10, 3),
        ),
        (
            "minimum-maximum",
            Length::vmin(50.0),
            Length::vmax(25.0),
            12,
            24,
            ratatui::layout::Rect::new(0, 0, 6, 6),
        ),
    ];

    for (name, width, height, viewport_width, viewport_height, expected) in cases {
        let root = text(name)
            .with_inline_style(
                TuiStyle::new()
                    .box_sizing(BoxSizing::BorderBox)
                    .size(LayoutSize::new(width.into(), height.into())),
            )
            .into_view();

        let _terminal = render_layout_root(&root, viewport_width, viewport_height)?;

        assert_eq!(retained_border_box(&root), expected, "case: {name}");
    }
    Ok(())
}

/// Verifies box sizing includes or excludes padding and borders predictably.
///
/// # Example Under Test
///
/// ```text
/// authored size: 8x6
/// padding: 1 cell on every side
/// border: 1 cell on every side
/// box_sizing: content-box or border-box
/// ```
///
/// # Assertions
///
/// - Content-box sizing adds padding and borders to the authored dimensions.
/// - Border-box sizing keeps padding and borders inside the authored dimensions.
/// - Both modes retain the expected content box.
#[test]
fn content_and_border_box_sizes_resolve_from_table() -> Result<()> {
    let cases = [
        (
            "content-box",
            BoxSizing::ContentBox,
            ratatui::layout::Rect::new(0, 0, 12, 10),
            ratatui::layout::Rect::new(2, 2, 8, 6),
        ),
        (
            "border-box",
            BoxSizing::BorderBox,
            ratatui::layout::Rect::new(0, 0, 8, 6),
            ratatui::layout::Rect::new(2, 2, 4, 2),
        ),
    ];

    for (name, box_sizing, expected_border, expected_content) in cases {
        let root = block(text(name))
            .with_inline_style(
                TuiStyle::new()
                    .box_sizing(box_sizing)
                    .size(LayoutSize::new(
                        Dimension::from(Length::cells(8.0)),
                        Dimension::from(Length::cells(6.0)),
                    ))
                    .padding(TuiSpacing::uniform(1)),
            )
            .into_view();

        let _terminal = render_layout_root(&root, 20, 12)?;
        let geometry = root
            .style_metadata()
            .and_then(StyleMetadata::layout_geometry)
            .expect("retained layout geometry");

        assert_eq!(geometry.border_box, expected_border, "case: {name}");
        assert_eq!(geometry.content_box, expected_content, "case: {name}");
    }
    Ok(())
}

/// Verifies fractional sibling sizes use cumulative terminal-cell rounding.
///
/// # Example Under Test
///
/// ```text
/// flex row width: 10
/// child widths: 33.333%, 33.333%, 33.333%
/// ```
///
/// # Assertions
///
/// - Rounded child widths are three, four, and three cells.
/// - Rounded child positions remain contiguous without gaps or overlap.
/// - The three rounded widths cover the ten-cell parent.
#[test]
fn fractional_percentages_round_cumulatively_to_terminal_cells() -> Result<()> {
    let child_style = TuiStyle::new()
        .box_sizing(BoxSizing::BorderBox)
        .size(LayoutSize::new(
            Dimension::from(Length::percent(33.333)),
            Dimension::from(Length::cells(1.0)),
        ))
        .flex_shrink(0.0);
    let root = div((
        text("A").with_inline_style(child_style.clone()),
        text("B").with_inline_style(child_style.clone()),
        text("C").with_inline_style(child_style),
    ))
    .with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(10.0)),
                Dimension::from(Length::cells(1.0)),
            )),
    )
    .into_view();

    let _terminal = render_layout_root(&root, 10, 1)?;

    assert_eq!(
        retained_div_child_border_box(&root, 0),
        ratatui::layout::Rect::new(0, 0, 3, 1)
    );
    assert_eq!(
        retained_div_child_border_box(&root, 1),
        ratatui::layout::Rect::new(3, 0, 4, 1)
    );
    assert_eq!(
        retained_div_child_border_box(&root, 2),
        ratatui::layout::Rect::new(7, 0, 3, 1)
    );
    Ok(())
}
