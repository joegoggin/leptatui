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

/// Verifies active selectors match active metadata with pseudo-class specificity.
///
/// # Example Under Test
///
/// ```text
/// A:active
/// ```
///
/// # Assertions
///
/// - An active anchor matches the compound selector.
/// - An inactive anchor does not match the compound selector.
/// - The selector contributes one pseudo-class and one type specificity unit.
#[test]
fn active_selector_matches_metadata_and_counts_as_pseudo_specificity() {
    let selector = StyleSelector::compound(vec![
        StyleSelector::view_type(ViewType::A),
        StyleSelector::active(),
    ]);
    let mut active = StyleMetadata::new(ViewType::A);
    active.set_active(true);
    let inactive = StyleMetadata::new(ViewType::A);

    let stylesheet = Stylesheet::new().rule(
        selector.clone(),
        TuiStyle::new().foreground(Color::LightCyan),
    );
    let active_style = stylesheet.resolve(
        &active,
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let inactive_style = stylesheet.resolve(
        &inactive,
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(active_style.foreground, Some(Color::LightCyan));
    assert_eq!(inactive_style.foreground, Some(Color::Blue));
    assert_eq!(selector.css_specificity(), (0, 1, 1));
}

/// Verifies insert selectors match insert metadata with pseudo-class specificity.
///
/// # Example Under Test
///
/// ```text
/// Input:insert
/// ```
///
/// # Assertions
///
/// - Insert-mode input metadata matches the compound selector.
/// - Normal-mode input metadata does not match the compound selector.
/// - The selector contributes one pseudo-class and one type specificity unit.
#[test]
fn insert_selector_matches_metadata_and_counts_as_pseudo_specificity() {
    let selector = StyleSelector::compound(vec![
        StyleSelector::view_type(ViewType::Input),
        StyleSelector::insert(),
    ]);
    let mut insert = StyleMetadata::new(ViewType::Input);
    insert.set_insert(true);
    let normal = StyleMetadata::new(ViewType::Input);
    let stylesheet = Stylesheet::new().rule(
        selector.clone(),
        TuiStyle::new().foreground(Color::Magenta),
    );

    let insert_style = stylesheet.resolve(
        &insert,
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let normal_style = stylesheet.resolve(
        &normal,
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(insert_style.foreground, Some(Color::Magenta));
    assert_eq!(normal_style.foreground, Some(Color::White));
    assert_eq!(selector.css_specificity(), (0, 1, 1));
}

/// Verifies visual selectors match visual metadata with pseudo-class specificity.
///
/// # Example Under Test
///
/// ```text
/// TextArea:visual
/// ```
///
/// # Assertions
///
/// - Visual-mode text-area metadata matches the compound selector.
/// - Normal-mode text-area metadata does not match the compound selector.
/// - The selector contributes one pseudo-class and one type specificity unit.
#[test]
fn visual_selector_matches_metadata_and_counts_as_pseudo_specificity() {
    let selector = StyleSelector::compound(vec![
        StyleSelector::view_type(ViewType::TextArea),
        StyleSelector::visual(),
    ]);
    let mut visual = StyleMetadata::new(ViewType::TextArea);
    visual.set_visual(true);
    let normal = StyleMetadata::new(ViewType::TextArea);
    let stylesheet = Stylesheet::new().rule(
        selector.clone(),
        TuiStyle::new().foreground(Color::LightMagenta),
    );

    let visual_style = stylesheet.resolve(
        &visual,
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let normal_style = stylesheet.resolve(
        &normal,
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(visual_style.foreground, Some(Color::LightMagenta));
    assert_eq!(normal_style.foreground, Some(Color::White));
    assert_eq!(selector.css_specificity(), (0, 1, 1));
}

/// Verifies visited selectors match visited metadata with pseudo-class specificity.
///
/// # Example Under Test
///
/// ```text
/// Link:visited
/// ```
///
/// # Assertions
///
/// - Visited link metadata matches the compound selector.
/// - Unvisited link metadata does not match the compound selector.
/// - The selector contributes one pseudo-class and one type specificity unit.
#[test]
fn visited_selector_matches_metadata_and_counts_as_pseudo_specificity() {
    let selector = StyleSelector::compound(vec![
        StyleSelector::view_type(ViewType::Link),
        StyleSelector::visited(),
    ]);
    let mut visited = StyleMetadata::new(ViewType::Link);
    visited.set_visited(true);
    let unvisited = StyleMetadata::new(ViewType::Link);
    let stylesheet = Stylesheet::new().rule(
        selector.clone(),
        TuiStyle::new().foreground(Color::LightMagenta),
    );

    let visited_style = stylesheet.resolve(
        &visited,
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let unvisited_style = stylesheet.resolve(
        &unvisited,
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(visited_style.foreground, Some(Color::LightMagenta));
    assert_eq!(unvisited_style.foreground, Some(Color::Blue));
    assert_eq!(selector.css_specificity(), (0, 1, 1));
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
