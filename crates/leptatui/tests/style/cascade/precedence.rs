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

/// Verifies standard views resolve their built-in presentation defaults.
///
/// # Example Under Test
///
/// ```text
/// Button, focused Input, insert-mode TextArea, visual-mode Input, Table,
/// ProgressBar, Link
/// ```
///
/// # Assertions
///
/// - Controls use white text, rounded borders, and horizontal padding.
/// - Focused controls use white on dark gray with bold text and thick borders.
/// - Insert-mode editable controls use yellow on dark gray.
/// - Visual-mode editable controls use magenta on dark gray.
/// - Visual mode takes precedence when insert and visual flags are both set.
/// - Unfocused visual-mode editable controls retain a magenta foreground.
/// - Regular links are blue and underlined.
/// - Focused unvisited links remain blue on the focus background.
/// - Visited links are magenta and remain magenta on the focus background.
/// - Tables and progress bars use their approved default palettes.
#[test]
fn standard_views_resolve_built_in_presentation_defaults() {
    let theme = ThemeVariables::new();
    let resolve = |metadata: &StyleMetadata| {
        Stylesheet::new().resolve(metadata, &[], TuiStyle::new(), &theme)
    };

    let button = resolve(&StyleMetadata::new(ViewType::Button));
    assert_eq!(button.foreground, Some(Color::White));
    assert_eq!(button.borders, Some(Borders::ALL));
    assert_eq!(button.border_type, Some(BorderType::Rounded));
    assert_eq!(button.padding, Some(TuiSpacing::horizontal(1)));

    let mut focused_input = StyleMetadata::new(ViewType::Input);
    focused_input.set_focused(true);
    let focused_input = resolve(&focused_input);
    assert_eq!(focused_input.foreground, Some(Color::White));
    assert_eq!(focused_input.background, Some(Color::DarkGray));
    assert_eq!(focused_input.modifiers, Some(Modifier::BOLD));
    assert_eq!(focused_input.border_type, Some(BorderType::Thick));

    let mut insert_text_area = StyleMetadata::new(ViewType::TextArea);
    insert_text_area.set_focused(true);
    insert_text_area.set_insert(true);
    let insert_text_area = resolve(&insert_text_area);
    assert_eq!(insert_text_area.foreground, Some(Color::Yellow));
    assert_eq!(insert_text_area.background, Some(Color::DarkGray));
    assert_eq!(insert_text_area.modifiers, Some(Modifier::BOLD));
    assert_eq!(insert_text_area.border_type, Some(BorderType::Thick));

    let mut visual_input = StyleMetadata::new(ViewType::Input);
    visual_input.set_focused(true);
    visual_input.set_insert(true);
    visual_input.set_visual(true);
    let visual_input = resolve(&visual_input);
    assert_eq!(visual_input.foreground, Some(Color::Magenta));
    assert_eq!(visual_input.background, Some(Color::DarkGray));
    assert_eq!(visual_input.modifiers, Some(Modifier::BOLD));
    assert_eq!(visual_input.border_type, Some(BorderType::Thick));

    let mut unfocused_visual_text_area = StyleMetadata::new(ViewType::TextArea);
    unfocused_visual_text_area.set_visual(true);
    let unfocused_visual_text_area = resolve(&unfocused_visual_text_area);
    assert_eq!(unfocused_visual_text_area.foreground, Some(Color::Magenta));
    assert_eq!(unfocused_visual_text_area.background, None);

    let table = resolve(&StyleMetadata::new(ViewType::Table));
    assert_eq!(table.foreground, Some(Color::White));

    let progress = resolve(&StyleMetadata::new(ViewType::ProgressBar));
    assert_eq!(progress.foreground, Some(Color::LightGreen));
    assert_eq!(progress.background, Some(Color::DarkGray));

    let link = resolve(&StyleMetadata::new(ViewType::Link));
    assert_eq!(link.foreground, Some(Color::Blue));
    assert_eq!(link.modifiers, Some(Modifier::UNDERLINED));

    let mut focused_link = StyleMetadata::new(ViewType::Link);
    focused_link.set_focused(true);
    let focused_link = resolve(&focused_link);
    assert_eq!(focused_link.foreground, Some(Color::Blue));
    assert_eq!(focused_link.background, Some(Color::DarkGray));
    assert_eq!(focused_link.modifiers, Some(Modifier::UNDERLINED));

    let mut visited_link = StyleMetadata::new(ViewType::Link);
    visited_link.set_visited(true);
    let visited_link = resolve(&visited_link);
    assert_eq!(visited_link.foreground, Some(Color::Magenta));
    assert_eq!(visited_link.modifiers, Some(Modifier::UNDERLINED));

    let mut focused_visited_link = StyleMetadata::new(ViewType::Link);
    focused_visited_link.set_focused(true);
    focused_visited_link.set_visited(true);
    let focused_visited_link = resolve(&focused_visited_link);
    assert_eq!(focused_visited_link.foreground, Some(Color::Magenta));
    assert_eq!(focused_visited_link.background, Some(Color::DarkGray));
    assert_eq!(
        focused_visited_link.modifiers,
        Some(Modifier::UNDERLINED)
    );
}

/// Verifies authored insert styles override editable-control defaults.
///
/// # Example Under Test
///
/// ```text
/// Input:insert { fg: Magenta, bg: Blue, border_type: Plain }
/// ```
///
/// # Assertions
///
/// - The authored foreground overrides yellow.
/// - The authored background overrides white.
/// - The authored border type overrides the thick focused border.
/// - A media rule can remove built-in control padding.
#[test]
fn authored_insert_and_media_rules_override_control_defaults() {
    let mut metadata = StyleMetadata::new(ViewType::Input);
    metadata.set_focused(true);
    metadata.set_insert(true);
    let stylesheet = stylesheet! {
        Input:insert => {
            fg: Color::Magenta,
            bg: Color::Blue,
            border_type: BorderType::Plain
        }
        @media (max-width: 20) {
            Input => { padding: TuiSpacing::ZERO }
        }
    };

    let resolved = stylesheet.resolve_for_viewport(
        &metadata,
        &[],
        TuiStyle::new(),
        ViewportSize::new(20, 5),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Magenta));
    assert_eq!(resolved.background, Some(Color::Blue));
    assert_eq!(resolved.border_type, Some(BorderType::Plain));
    assert_eq!(resolved.padding, Some(TuiSpacing::ZERO));
}

/// Verifies authored visual styles override editable-control defaults.
///
/// # Example Under Test
///
/// ```text
/// TextArea:visual { fg: Cyan, bg: Blue, border_type: Plain }
/// ```
///
/// # Assertions
///
/// - The authored foreground overrides magenta.
/// - The authored background overrides dark gray.
/// - The authored border type overrides the thick focused border.
#[test]
fn authored_visual_rules_override_control_defaults() {
    let mut metadata = StyleMetadata::new(ViewType::TextArea);
    metadata.set_focused(true);
    metadata.set_visual(true);
    let stylesheet = stylesheet! {
        TextArea:visual => {
            fg: Color::Cyan,
            bg: Color::Blue,
            border_type: BorderType::Plain
        }
    };

    let resolved = stylesheet.resolve(
        &metadata,
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Cyan));
    assert_eq!(resolved.background, Some(Color::Blue));
    assert_eq!(resolved.border_type, Some(BorderType::Plain));
}

/// Verifies route-anchor defaults remain below authored cascade declarations.
///
/// # Example Under Test
///
/// ```text
/// A default
/// A { fg: red }
/// .accent { fg: green }
/// inline fg: yellow
/// A:focus { bg: blue }
/// A:active { fg: magenta }
/// ```
///
/// # Assertions
///
/// - The route-anchor default foreground is blue.
/// - A type rule overrides the default foreground.
/// - A class rule overrides the type rule.
/// - An inline declaration overrides the class rule.
/// - A focus rule overrides the default focus background.
/// - The active defaults remain blue, bold, and underlined.
/// - An active rule overrides the default active foreground.
#[test]
fn route_anchor_defaults_have_low_cascade_precedence() {
    let theme = ThemeVariables::new();
    let plain = StyleMetadata::new(ViewType::A);
    let default_style = Stylesheet::new().resolve(&plain, &[], TuiStyle::new(), &theme);
    assert_eq!(default_style.foreground, Some(Color::Blue));

    let type_stylesheet = stylesheet! { A => { fg: Color::Red } };
    let type_style = type_stylesheet.resolve(&plain, &[], TuiStyle::new(), &theme);
    assert_eq!(type_style.foreground, Some(Color::Red));

    let mut class = StyleMetadata::new(ViewType::A);
    class.set_classes("accent");
    let class_stylesheet = stylesheet! {
        A => { fg: Color::Red }
        .accent => { fg: Color::Green }
    };
    let class_style = class_stylesheet.resolve(&class, &[], TuiStyle::new(), &theme);
    assert_eq!(class_style.foreground, Some(Color::Green));

    class.set_inline_style(TuiStyle::new().foreground(Color::Yellow));
    let inline_style = class_stylesheet.resolve(&class, &[], TuiStyle::new(), &theme);
    assert_eq!(inline_style.foreground, Some(Color::Yellow));

    class.set_focused(true);
    let focus_stylesheet = stylesheet! { A:focus => { bg: Color::Blue } };
    let focus_style = focus_stylesheet.resolve(&class, &[], TuiStyle::new(), &theme);
    assert_eq!(focus_style.background, Some(Color::Blue));

    let mut active = StyleMetadata::new(ViewType::A);
    active.set_active(true);
    let active_default = Stylesheet::new().resolve(&active, &[], TuiStyle::new(), &theme);
    assert_eq!(active_default.foreground, Some(Color::Blue));
    assert_eq!(
        active_default.modifiers,
        Some(Modifier::BOLD | Modifier::UNDERLINED)
    );

    let active_stylesheet = stylesheet! { A:active => { fg: Color::Magenta } };
    let active_style = active_stylesheet.resolve(&active, &[], TuiStyle::new(), &theme);
    assert_eq!(active_style.foreground, Some(Color::Magenta));
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
