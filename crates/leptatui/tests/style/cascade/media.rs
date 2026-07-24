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

/// Verifies stylesheet flex-direction declarations resolve into view styles.
///
/// # Example Under Test
///
/// ```text
/// .controls { flex_direction: Column }
/// ```
///
/// # Assertions
///
/// - View metadata is available for stylesheet resolution.
/// - The resolved flex direction is column.
#[test]
fn stylesheet_flex_direction_declaration_resolves() {
    let view = text("Controls").with_classes("controls");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("controls"),
        TuiStyle::new().flex_direction(FlexDirection::Column),
    );

    let resolved = stylesheet.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.flex_direction, Some(FlexDirection::Column));
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

/// Verifies media rules can override flex direction by viewport.
///
/// # Example Under Test
///
/// ```text
/// .controls { flex_direction: Row }
/// @media (max-width: 60) { .controls { flex_direction: Column } }
/// ```
///
/// # Assertions
///
/// - The compact viewport resolves column flex direction.
/// - The wide viewport resolves row flex direction.
#[test]
fn stylesheet_media_query_can_override_flex_direction() {
    let view = text("Controls").with_classes("controls");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::class("controls"),
            TuiStyle::new().flex_direction(FlexDirection::Row),
        )
        .media_rule(
            MediaQuery::max_width(60),
            StyleSelector::class("controls"),
            TuiStyle::new().flex_direction(FlexDirection::Column),
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

    assert_eq!(compact.flex_direction, Some(FlexDirection::Column));
    assert_eq!(wide.flex_direction, Some(FlexDirection::Row));
}
