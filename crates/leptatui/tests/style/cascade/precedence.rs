/// Verifies important stylesheet flex direction overrides inline flex direction.
///
/// # Example Under Test
///
/// ```text
/// inline flex_direction: Row
/// .controls { flex_direction: Column !important }
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved flex direction is column.
///
/// # Why
///
/// Important stylesheet declarations outrank normal inline declarations.
#[test]
fn stylesheet_important_flex_direction_overrides_inline_flex_direction() {
    let view = text("Controls")
        .with_classes("controls")
        .with_inline_style(TuiStyle::new().flex_direction(FlexDirection::Row));
    let stylesheet = stylesheet! {
        .controls => { flex_direction: FlexDirection::Column !important }
    };

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.flex_direction, Some(FlexDirection::Column));
}

/// Verifies inline styles override normal stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// text("Save").with_id("save").with_inline_style(black)
/// Stylesheet::new().rule(StyleSelector::id("save"), green)
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color is black.
///
/// # Why
///
/// Normal inline styles override normal stylesheet declarations.
#[test]
fn inline_style_overrides_stylesheet_rules() {
    let view = text("Save")
        .with_id("save")
        .with_inline_style(TuiStyle::new().foreground(Color::Black));
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::id("save"),
        TuiStyle::new().foreground(Color::Green),
    );

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Black));
}

/// Verifies inherited text styles remain unless the view overrides them.
///
/// # Example Under Test
///
/// ```text
/// text("Child").with_inline_style(foreground: yellow)
/// inherited = foreground: green, modifier: bold
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color is yellow.
/// - The resolved modifier is bold.
///
/// # Why
///
/// Child styles should preserve inherited fields that are not locally set.
#[test]
fn inherited_text_styles_flow_to_children_unless_overridden() {
    let view = text("Child").with_inline_style(TuiStyle::new().foreground(Color::Yellow));
    let inherited = TuiStyle::new()
        .foreground(Color::Green)
        .modifier(Modifier::BOLD);

    let resolved = Stylesheet::new().resolve(
        view.style_metadata().unwrap(),
        &[],
        inherited,
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Yellow));
    assert_eq!(resolved.modifiers, Some(Modifier::BOLD));
}

/// Verifies important stylesheet values override normal inline styles.
///
/// # Example Under Test
///
/// ```text
/// inline fg: Black
/// .alert { fg: Red !important }
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color is red.
#[test]
fn stylesheet_important_overrides_normal_inline_style() {
    let view = text("Alert")
        .with_classes("alert")
        .with_inline_style(TuiStyle::new().foreground(Color::Black));
    let stylesheet = stylesheet! {
        .alert => { fg: Color::Red !important }
    };

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Red));
}

/// Verifies important declarations override normal higher-specificity rules.
///
/// # Example Under Test
///
/// ```text
/// Text { fg: Red !important }
/// .alert { fg: Blue }
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color comes from the important type rule.
///
/// # Why
///
/// Importance is compared before selector specificity.
#[test]
fn stylesheet_important_overrides_normal_higher_specificity_rule() {
    let view = text("Alert").with_classes("alert");
    let stylesheet = stylesheet! {
        Text => { fg: Color::Red !important }
        .alert => { fg: Color::Blue }
    };

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Red));
}

/// Verifies important rules use specificity and source order within importance.
///
/// # Example Under Test
///
/// ```text
/// Button:focus { fg: Green !important }
/// :focus { fg: Yellow !important }
/// .primary { bg: Blue !important }
/// .danger { bg: Red !important }
/// ```
///
/// # Assertions
///
/// - The foreground comes from the more specific important rule.
/// - The background comes from the later equal-specificity important rule.
#[test]
fn stylesheet_important_rules_use_specificity_then_source_order() {
    let view = button("Save")
        .with_classes("primary danger")
        .with_focus(true);
    let stylesheet = stylesheet! {
        Button:focus => { fg: Color::Green !important }
        :focus => { fg: Color::Yellow !important }
        .primary => { bg: Color::Blue !important }
        .danger => { bg: Color::Red !important }
    };

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Green));
    assert_eq!(resolved.background, Some(Color::Red));
}

/// Verifies normal declarations do not override important mixin values.
///
/// # Example Under Test
///
/// ```text
/// @mixin urgent { fg: Red !important }
/// .alert { @include urgent, fg: Blue }
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color remains red.
///
/// # Why
///
/// Mixin expansion should preserve declaration importance while merging.
#[test]
fn stylesheet_normal_declaration_does_not_override_important_mixin_value() {
    let view = text("Alert").with_classes("alert");
    let stylesheet = stylesheet! {
        @mixin urgent {
            fg: Color::Red !important
        }

        .alert => { @include urgent, fg: Color::Blue }
    };

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Red));
}

/// Verifies important layout declarations override every normal inline layout value.
///
/// # Example Under Test
///
/// ```text
/// inline: display Flex, aspect_ratio 1, position Relative, z_index Auto
/// .layout: display Grid, aspect_ratio 1.5, position Absolute,
///          z_index 7 !important
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - Every grouped box, sizing, flexbox, alignment, grid, and positioning
///   property resolves from the important stylesheet rule.
///
/// # Why
///
/// Layout properties must participate independently in the same importance
/// pipeline as existing visual declarations.
#[test]
fn stylesheet_important_layout_properties_override_inline_values() {
    let line = GridLine::new(GridPlacement::line(2), GridPlacement::span(3));
    let inline_template = vec![GridTemplateTrack::from(GridTrackSize::Auto)];
    let important_template = vec![GridTemplateTrack::repeat(
        GridRepeat::count(2),
        vec![GridTrackSize::from(Fraction::new(1.0))],
    )];
    let inline_auto_tracks = vec![GridTrackSize::Auto];
    let important_auto_tracks = vec![GridTrackSize::minmax(
        GridMinTrackSize::MinContent,
        GridMaxTrackSize::MaxContent,
    )];
    let inline = TuiStyle::new()
        .display(Display::Flex)
        .box_sizing(BoxSizing::ContentBox)
        .overflow(Axes::all(Overflow::Visible))
        .size(LayoutSize::all(Dimension::Auto))
        .min_size(LayoutSize::all(Dimension::Auto))
        .max_size(LayoutSize::all(Dimension::Auto))
        .aspect_ratio(1.0)
        .margin(Edges::all(LengthAuto::Auto))
        .gap(Axes::all(Length::cells(0.0)))
        .flex_direction(FlexDirection::Row)
        .flex_wrap(FlexWrap::NoWrap)
        .flex_basis(Dimension::Auto)
        .flex_grow(0.0)
        .flex_shrink(1.0)
        .align_items(AlignItems::Stretch)
        .align_self(AlignSelf::Auto)
        .align_content(AlignContent::Stretch)
        .justify_items(JustifyItems::Stretch)
        .justify_self(JustifySelf::Auto)
        .justify_content(JustifyContent::Start)
        .grid_auto_flow(GridAutoFlow::Row)
        .grid_template_rows(inline_template.clone())
        .grid_template_columns(inline_template)
        .grid_auto_rows(inline_auto_tracks.clone())
        .grid_auto_columns(inline_auto_tracks)
        .grid_row(GridLine::default())
        .grid_column(GridLine::default())
        .position(Position::Relative)
        .inset(Edges::all(LengthAuto::Auto))
        .z_index(ZIndex::Auto);
    let view = text("Layout")
        .with_classes("layout")
        .with_inline_style(inline);
    let stylesheet = stylesheet! {
        .layout => {
            display: Display::Grid !important,
            box_sizing: BoxSizing::BorderBox !important,
            overflow: Axes::new(Overflow::Hidden, Overflow::Auto) !important,
            size: LayoutSize::new(Dimension::MinContent, Dimension::MaxContent) !important,
            min_size: LayoutSize::all(Dimension::from(Length::cells(2.0))) !important,
            max_size: LayoutSize::all(Dimension::FitContent(Length::cells(40.0))) !important,
            aspect_ratio: 1.5 !important,
            margin: Edges::all(LengthAuto::from(Length::cells(1.0))) !important,
            gap: Axes::new(Length::cells(2.0), Length::cells(3.0)) !important,
            flex_direction: FlexDirection::ColumnReverse !important,
            flex_wrap: FlexWrap::WrapReverse !important,
            flex_basis: Dimension::from(Length::percent(25.0)) !important,
            flex_grow: 2.0 !important,
            flex_shrink: 0.5 !important,
            align_items: AlignItems::Center !important,
            align_self: AlignSelf::FlexEnd !important,
            align_content: AlignContent::SpaceAround !important,
            justify_items: JustifyItems::End !important,
            justify_self: JustifySelf::Center !important,
            justify_content: JustifyContent::SpaceEvenly !important,
            grid_auto_flow: GridAutoFlow::ColumnDense !important,
            grid_template_rows: important_template.clone() !important,
            grid_template_columns: important_template.clone() !important,
            grid_auto_rows: important_auto_tracks.clone() !important,
            grid_auto_columns: important_auto_tracks.clone() !important,
            grid_row: line !important,
            grid_column: line !important,
            position: Position::Absolute !important,
            inset: Edges::symmetric(LengthAuto::Auto, Length::cells(4.0).into()) !important,
            z_index: ZIndex::Integer(7) !important
        }
    };

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let expected = TuiStyle::new()
        .display(Display::Grid)
        .box_sizing(BoxSizing::BorderBox)
        .overflow(Axes::new(Overflow::Hidden, Overflow::Auto))
        .size(LayoutSize::new(
            Dimension::MinContent,
            Dimension::MaxContent,
        ))
        .min_size(LayoutSize::all(Dimension::from(Length::cells(2.0))))
        .max_size(LayoutSize::all(Dimension::FitContent(Length::cells(40.0))))
        .aspect_ratio(1.5)
        .margin(Edges::all(Length::cells(1.0).into()))
        .gap(Axes::new(Length::cells(2.0), Length::cells(3.0)))
        .flex_direction(FlexDirection::ColumnReverse)
        .flex_wrap(FlexWrap::WrapReverse)
        .flex_basis(Dimension::from(Length::percent(25.0)))
        .flex_grow(2.0)
        .flex_shrink(0.5)
        .align_items(AlignItems::Center)
        .align_self(AlignSelf::FlexEnd)
        .align_content(AlignContent::SpaceAround)
        .justify_items(JustifyItems::End)
        .justify_self(JustifySelf::Center)
        .justify_content(JustifyContent::SpaceEvenly)
        .grid_auto_flow(GridAutoFlow::ColumnDense)
        .grid_template_rows(important_template.clone())
        .grid_template_columns(important_template)
        .grid_auto_rows(important_auto_tracks.clone())
        .grid_auto_columns(important_auto_tracks)
        .grid_row(line)
        .grid_column(line)
        .position(Position::Absolute)
        .inset(Edges::symmetric(
            LengthAuto::Auto,
            Length::cells(4.0).into(),
        ))
        .z_index(ZIndex::Integer(7));

    assert_eq!(resolved, expected);
}

/// Verifies layout declarations use selector specificity and source order.
///
/// # Example Under Test
///
/// ```text
/// Text { display: Block }
/// .panel { display: Flex }
/// .panel { display: Grid }
/// ```
///
/// # Assertions
///
/// - The class selector overrides the less-specific type selector.
/// - The later equal-specificity class rule supplies the resolved display.
#[test]
fn stylesheet_layout_properties_use_specificity_and_source_order() {
    let view = text("Panel").with_classes("panel");
    let stylesheet = stylesheet! {
        Text => { display: Display::Block }
        .panel => { display: Display::Flex }
        .panel => { display: Display::Grid }
    };

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.display, Some(Display::Grid));
}
