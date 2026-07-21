/// Verifies class stylesheet rules override type stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// text("Save").with_classes("primary")
/// Stylesheet::new()
///     .rule(StyleSelector::view_type(ViewType::Text), white)
///     .rule(StyleSelector::class("primary"), yellow)
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color is yellow.
///
/// # Why
///
/// Class selectors should have higher specificity than type selectors.
#[test]
fn stylesheet_class_overrides_type_style() {
    let view = text("Save").with_classes("primary");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::view_type(ViewType::Text),
            TuiStyle::new().foreground(Color::White),
        )
        .rule(
            StyleSelector::class("primary"),
            TuiStyle::new().foreground(Color::Yellow),
        );

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Yellow));
}

/// Verifies id stylesheet rules override class stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// text("Save").with_classes("primary").with_id("save")
/// Stylesheet::new()
///     .rule(StyleSelector::class("primary"), yellow)
///     .rule(StyleSelector::id("save"), green)
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color is green.
///
/// # Why
///
/// Id selectors should have higher specificity than class selectors.
#[test]
fn stylesheet_id_overrides_class_style() {
    let view = text("Save").with_classes("primary").with_id("save");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::class("primary"),
            TuiStyle::new().foreground(Color::Yellow),
        )
        .rule(
            StyleSelector::id("save"),
            TuiStyle::new().foreground(Color::Green),
        );

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Green));
}

/// Verifies selector specificity sums compound and descendant selector parts.
///
/// # Example Under Test
///
/// ```text
/// .panel Button:focus
/// ```
///
/// # Assertions
///
/// - The selector has no id specificity.
/// - The selector has two class-or-pseudo specificity units.
/// - The selector has one type specificity unit.
#[test]
fn selector_css_specificity_sums_compound_and_descendant_parts() {
    let selector = StyleSelector::descendant(
        vec![StyleSelector::class("panel")],
        StyleSelector::compound(vec![
            StyleSelector::view_type(ViewType::Button),
            StyleSelector::focus(),
        ]),
    );

    assert_eq!(selector.css_specificity(), (0, 2, 1));
}

/// Verifies descendant selector specificity overrides later class rules.
///
/// # Example Under Test
///
/// ```text
/// .panel Text
/// .label
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color comes from the descendant rule.
///
/// # Why
///
/// Descendant selector specificity should participate in normal cascade order.
#[test]
fn stylesheet_descendant_specificity_overrides_later_class_rule() {
    let mut panel = StyleMetadata::new(ViewType::Block);
    panel.set_classes("panel");
    let view = text("Save").with_classes("label");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::descendant(
                vec![StyleSelector::class("panel")],
                StyleSelector::view_type(ViewType::Text),
            ),
            TuiStyle::new().foreground(Color::Green),
        )
        .rule(
            StyleSelector::class("label"),
            TuiStyle::new().foreground(Color::Yellow),
        );

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[panel],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Green));
}

/// Verifies compound selector specificity overrides later pseudo rules.
///
/// # Example Under Test
///
/// ```text
/// Button:focus
/// :focus
/// ```
///
/// # Assertions
///
/// - Focused button metadata is available for stylesheet resolution.
/// - The resolved foreground color comes from the compound rule.
///
/// # Why
///
/// Type plus pseudo specificity should outrank a pseudo-only rule.
#[test]
fn stylesheet_compound_specificity_overrides_later_pseudo_rule() {
    let view = button("Save").with_focus(true);
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::compound(vec![
                StyleSelector::view_type(ViewType::Button),
                StyleSelector::focus(),
            ]),
            TuiStyle::new().foreground(Color::Green),
        )
        .rule(
            StyleSelector::focus(),
            TuiStyle::new().foreground(Color::Yellow),
        );

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Green));
}

/// Verifies equal-specificity rules are resolved by source order.
///
/// # Example Under Test
///
/// ```text
/// .primary
/// .warning
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color comes from the later matching rule.
#[test]
fn stylesheet_equal_specificity_uses_source_order() {
    let view = text("Save").with_classes("primary warning");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::class("primary"),
            TuiStyle::new().foreground(Color::Green),
        )
        .rule(
            StyleSelector::class("warning"),
            TuiStyle::new().foreground(Color::Yellow),
        );

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Yellow));
}

/// Verifies media rules match the provided viewport size.
///
/// # Example Under Test
///
/// ```text
/// @media (max-width: 80) { .compact }
/// ```
///
/// # Assertions
///
/// - A viewport at width `80` resolves the media-rule color.
/// - A viewport at width `81` resolves the base-rule color.
/// - Resolution without a viewport ignores media rules.
#[test]
fn stylesheet_media_query_matches_viewport_size() {
    let view = text("Save").with_classes("compact");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::class("compact"),
            TuiStyle::new().foreground(Color::White),
        )
        .media_rule(
            MediaQuery::max_width(80),
            StyleSelector::class("compact"),
            TuiStyle::new().foreground(Color::Yellow),
        );

    let compact = stylesheet.resolve_for_viewport(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(80, 24),
        &ThemeVariables::new(),
    );
    let wide = stylesheet.resolve_for_viewport(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(81, 24),
        &ThemeVariables::new(),
    );
    let without_viewport = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(compact.foreground, Some(Color::Yellow));
    assert_eq!(wide.foreground, Some(Color::White));
    assert_eq!(without_viewport.foreground, Some(Color::White));
}

/// Verifies media queries combine width and height conditions.
///
/// # Example Under Test
///
/// ```text
/// min-width: 80 and min-height: 24 and max-height: 40
/// ```
///
/// # Assertions
///
/// - A matching viewport resolves the media-rule background.
/// - A too-narrow viewport does not resolve the media-rule background.
/// - A too-tall viewport does not resolve the media-rule background.
#[test]
fn stylesheet_media_query_combines_width_and_height_conditions() {
    let view = text("Save");
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::min_width(80)
            .and(MediaQuery::min_height(24))
            .and(MediaQuery::max_height(40)),
        StyleSelector::view_type(ViewType::Text),
        TuiStyle::new().background(Color::Blue),
    );

    let matching = stylesheet.resolve_for_viewport(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(100, 30),
        &ThemeVariables::new(),
    );
    let too_narrow = stylesheet.resolve_for_viewport(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(79, 30),
        &ThemeVariables::new(),
    );
    let too_tall = stylesheet.resolve_for_viewport(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(100, 41),
        &ThemeVariables::new(),
    );

    assert_eq!(matching.background, Some(Color::Blue));
    assert_eq!(too_narrow.background, None);
    assert_eq!(too_tall.background, None);
}

/// Verifies matching media rules keep selector specificity ordering.
///
/// # Example Under Test
///
/// ```text
/// @media (max-width: 80) { #save }
/// @media (max-width: 80) { .warning }
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved foreground color comes from the id selector.
///
/// # Why
///
/// Media filtering should not flatten selector specificity.
#[test]
fn matching_media_rules_keep_selector_specificity() {
    let view = text("Save").with_id("save").with_classes("warning");
    let stylesheet = Stylesheet::new()
        .media_rule(
            MediaQuery::max_width(80),
            StyleSelector::id("save"),
            TuiStyle::new().foreground(Color::Green),
        )
        .media_rule(
            MediaQuery::max_width(80),
            StyleSelector::class("warning"),
            TuiStyle::new().foreground(Color::Yellow),
        );

    let resolved = stylesheet.resolve_for_viewport(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(80, 24),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.foreground, Some(Color::Green));
}

/// Verifies stylesheet direction declarations resolve into view styles.
///
/// # Example Under Test
///
/// ```text
/// .controls { direction: Column }
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved layout direction is column.
#[test]
fn stylesheet_direction_declaration_resolves() {
    let view = text("Controls").with_classes("controls");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("controls"),
        TuiStyle::new().direction(LayoutDirection::Column),
    );

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.direction, Some(LayoutDirection::Column));
}

/// Verifies stylesheet image size declarations resolve into view styles.
///
/// # Example Under Test
///
/// ```text
/// .thumbnail { image_size: TuiSize::new(24, 8) }
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved image size is the stylesheet-declared terminal-cell size.
#[test]
fn stylesheet_image_size_declaration_resolves() {
    let view = image("missing.png").with_classes("thumbnail");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("thumbnail"),
        TuiStyle::new().image_size(TuiSize::new(24, 8)),
    );

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.image_size, Some(TuiSize::new(24, 8)));
}

/// Verifies media rules can override layout direction by viewport.
///
/// # Example Under Test
///
/// ```text
/// .controls { direction: Row }
/// @media (max-width: 60) { .controls { direction: Column } }
/// ```
///
/// # Assertions
///
/// - The compact viewport resolves column direction.
/// - The wide viewport resolves row direction.
#[test]
fn stylesheet_media_query_can_override_layout_direction() {
    let view = text("Controls").with_classes("controls");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::class("controls"),
            TuiStyle::new().direction(LayoutDirection::Row),
        )
        .media_rule(
            MediaQuery::max_width(60),
            StyleSelector::class("controls"),
            TuiStyle::new().direction(LayoutDirection::Column),
        );

    let compact = stylesheet.resolve_for_viewport(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(60, 24),
        &ThemeVariables::new(),
    );
    let wide = stylesheet.resolve_for_viewport(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        ViewportSize::new(61, 24),
        &ThemeVariables::new(),
    );

    assert_eq!(compact.direction, Some(LayoutDirection::Column));
    assert_eq!(wide.direction, Some(LayoutDirection::Row));
}

/// Verifies important stylesheet direction overrides inline direction.
///
/// # Example Under Test
///
/// ```text
/// inline direction: Row
/// .controls { direction: Column !important }
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved layout direction is column.
///
/// # Why
///
/// Important stylesheet declarations outrank normal inline declarations.
#[test]
fn stylesheet_important_direction_overrides_inline_direction() {
    let view = text("Controls")
        .with_classes("controls")
        .with_inline_style(TuiStyle::new().direction(LayoutDirection::Row));
    let stylesheet = stylesheet! {
        .controls => { direction: LayoutDirection::Column !important }
    };

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.direction, Some(LayoutDirection::Column));
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
