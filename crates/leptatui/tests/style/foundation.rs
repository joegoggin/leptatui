/// Verifies terminal UI style maps to Ratatui style.
///
/// # Example Under Test
///
/// ```text
/// TuiStyle::new()
///     .foreground(Color::Yellow)
///     .background(Color::Black)
///     .modifier(Modifier::BOLD | Modifier::ITALIC)
/// ```
///
/// # Assertions
///
/// - The converted style has a yellow foreground.
/// - The converted style has a black background.
/// - The converted style has bold and italic modifiers.
#[test]
fn tui_style_maps_to_ratatui_style() {
    let style = TuiStyle::new()
        .foreground(Color::Yellow)
        .background(Color::Black)
        .modifier(Modifier::BOLD | Modifier::ITALIC);

    assert_eq!(
        style.to_ratatui_style(),
        Style::new()
            .fg(Color::Yellow)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC)
    );
}

/// Verifies terminal UI spacing maps to Ratatui padding.
///
/// # Example Under Test
///
/// ```text
/// TuiSpacing::new(1, 2, 3, 4)
/// ```
///
/// # Assertions
///
/// - Left padding is `1`.
/// - Right padding is `2`.
/// - Top padding is `3`.
/// - Bottom padding is `4`.
#[test]
fn tui_spacing_maps_to_ratatui_padding() {
    assert_eq!(
        Padding::from(TuiSpacing::new(1, 2, 3, 4)),
        Padding::new(1, 2, 3, 4)
    );
}

/// Verifies terminal UI style can build a configured Ratatui block.
///
/// # Example Under Test
///
/// ```text
/// TuiStyle::new()
///     .borders(Borders::ALL)
///     .border_type(BorderType::Rounded)
///     .padding(TuiSpacing::uniform(1))
/// ```
///
/// # Assertions
///
/// - A block can be built from a style with borders.
/// - A block can be built from a style with rounded border glyphs.
/// - A block can be built from a style with uniform padding.
#[test]
fn tui_style_builds_a_block_with_border_configuration() {
    let style = TuiStyle::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .padding(TuiSpacing::uniform(1));

    let _block = style.to_block();
}

/// Verifies terminal UI styles retain the complete authored layout surface.
///
/// # Example Under Test
///
/// ```text
/// TuiStyle::new()
///     .display(Display::Flex)
///     .size(LayoutSize::all(Dimension::Auto))
///     .position(Position::Absolute)
/// ```
///
/// # Assertions
///
/// - Every phase-13 layout builder stores its typed value.
/// - Flex growth and shrink factors retain floating-point values.
/// - Layout properties are removed from inherited descendant values.
#[test]
fn tui_style_stores_layout_properties_without_inheriting_them() {
    let grid_line = GridLine::new(GridPlacement::line(1), GridPlacement::span(2));
    let length = Length::cells(2.0);
    let style = TuiStyle::new()
        .display(Display::Flex)
        .box_sizing(BoxSizing::BorderBox)
        .overflow(Axes::new(Overflow::Hidden, Overflow::Auto))
        .size(LayoutSize::new(
            Dimension::from(Length::percent(100.0)),
            Dimension::Auto,
        ))
        .min_size(LayoutSize::all(Dimension::MinContent))
        .max_size(LayoutSize::all(Dimension::MaxContent))
        .margin(Edges::all(LengthAuto::from(length)))
        .gap(Axes::new(length, Length::cells(3.0)))
        .flex_direction(FlexDirection::Column)
        .flex_wrap(FlexWrap::Wrap)
        .flex_basis(Dimension::FitContent(Length::cells(12.0)))
        .flex_grow(2.0)
        .flex_shrink(0.5)
        .align_items(AlignItems::Center)
        .align_self(AlignSelf::FlexEnd)
        .align_content(AlignContent::SpaceBetween)
        .justify_items(JustifyItems::End)
        .justify_self(JustifySelf::Center)
        .justify_content(JustifyContent::SpaceEvenly)
        .grid_auto_flow(GridAutoFlow::RowDense)
        .grid_row(grid_line)
        .grid_column(grid_line)
        .position(Position::Absolute)
        .inset(Edges::symmetric(LengthAuto::Auto, length.into()))
        .z_index(ZIndex::Integer(4));

    assert_eq!(style.display, Some(Display::Flex));
    assert_eq!(style.box_sizing, Some(BoxSizing::BorderBox));
    assert_eq!(
        style.overflow,
        Some(Axes::new(Overflow::Hidden, Overflow::Auto))
    );
    assert_eq!(
        style.size,
        Some(LayoutSize::new(
            Dimension::from(Length::percent(100.0)),
            Dimension::Auto
        ))
    );
    assert_eq!(
        style.min_size,
        Some(LayoutSize::all(Dimension::MinContent))
    );
    assert_eq!(
        style.max_size,
        Some(LayoutSize::all(Dimension::MaxContent))
    );
    assert_eq!(style.margin, Some(Edges::all(length.into())));
    assert_eq!(style.gap, Some(Axes::new(length, Length::cells(3.0))));
    assert_eq!(style.flex_direction, Some(FlexDirection::Column));
    assert_eq!(style.flex_wrap, Some(FlexWrap::Wrap));
    assert_eq!(
        style.flex_basis,
        Some(Dimension::FitContent(Length::cells(12.0)))
    );
    assert_eq!(style.flex_grow, Some(2.0));
    assert_eq!(style.flex_shrink, Some(0.5));
    assert_eq!(style.align_items, Some(AlignItems::Center));
    assert_eq!(style.align_self, Some(AlignSelf::FlexEnd));
    assert_eq!(style.align_content, Some(AlignContent::SpaceBetween));
    assert_eq!(style.justify_items, Some(JustifyItems::End));
    assert_eq!(style.justify_self, Some(JustifySelf::Center));
    assert_eq!(style.justify_content, Some(JustifyContent::SpaceEvenly));
    assert_eq!(style.grid_auto_flow, Some(GridAutoFlow::RowDense));
    assert_eq!(style.grid_row, Some(grid_line));
    assert_eq!(style.grid_column, Some(grid_line));
    assert_eq!(style.position, Some(Position::Absolute));
    assert_eq!(
        style.inset,
        Some(Edges::symmetric(LengthAuto::Auto, length.into()))
    );
    assert_eq!(style.z_index, Some(ZIndex::Integer(4)));

    let inherited = style.inherited_values();
    assert_eq!(inherited, TuiStyle::new());
}
