/// Verifies stylesheet theme variables resolve against the active theme.
///
/// # Example Under Test
///
/// ```text
/// .status { fg: theme_color("text"), bg: theme_color("surface") }
/// ```
///
/// # Assertions
///
/// - Light theme resolution paints black on white.
/// - Dark theme resolution paints white on black.
#[test]
fn stylesheet_theme_variables_resolve_against_active_theme() {
    let view = text("Status").with_classes("status");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("status"),
        StyleDeclarations::new()
            .foreground(theme_color("text"))
            .background(theme_color("surface")),
    );
    let light = ThemeVariables::new()
        .color("text", Color::Black)
        .color("surface", Color::White);
    let dark = ThemeVariables::new()
        .color("text", Color::White)
        .color("surface", Color::Black);

    let light_style =
        stylesheet.resolve(view.style_metadata().unwrap(), &[], TuiStyle::new(), &light);
    let dark_style =
        stylesheet.resolve(view.style_metadata().unwrap(), &[], TuiStyle::new(), &dark);

    assert_eq!(light_style.foreground, Some(Color::Black));
    assert_eq!(light_style.background, Some(Color::White));
    assert_eq!(dark_style.foreground, Some(Color::White));
    assert_eq!(dark_style.background, Some(Color::Black));
}

/// Verifies buttons resolve built-in colors for blurred and focused states.
///
/// # Example Under Test
///
/// ```text
/// button("Cancel")
/// button("Save").with_focus(true)
/// ```
///
/// # Assertions
///
/// - The blurred button resolves to a white foreground with no background.
/// - The focused button resolves to a white foreground on a dark-gray background.
#[test]
fn buttons_resolve_default_focus_colors() {
    let blurred = button("Cancel");
    let focused = button("Save").with_focus(true);
    let stylesheet = Stylesheet::new();
    let theme = ThemeVariables::new();

    let blurred_style = stylesheet.resolve(
        blurred.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );
    let focused_style = stylesheet.resolve(
        focused.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &theme,
    );

    assert_eq!(blurred_style.foreground, Some(Color::White));
    assert_eq!(blurred_style.background, None);
    assert_eq!(focused_style.foreground, Some(Color::White));
    assert_eq!(focused_style.background, Some(Color::DarkGray));
}

/// Verifies focus selectors match only focused views.
///
/// # Example Under Test
///
/// ```text
/// button("Save").with_focus(true)
/// button("Cancel")
/// Stylesheet::new().rule(StyleSelector::focus(), yellow)
/// ```
///
/// # Assertions
///
/// - Focused button metadata is available for stylesheet resolution.
/// - Blurred button metadata is available for stylesheet resolution.
/// - The focused button resolves to a yellow foreground.
/// - The blurred button retains its default white foreground.
///
/// # Why
///
/// Focus styling should depend on view focus state, not just view type.
#[test]
fn stylesheet_focus_selector_matches_only_focused_views() {
    let focused = button("Save").with_focus(true);
    let blurred = button("Cancel");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::focus(),
        TuiStyle::new().foreground(Color::Yellow),
    );

    let focused_style = stylesheet.resolve(
        focused.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let blurred_style = stylesheet.resolve(
        blurred.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(focused_style.foreground, Some(Color::Yellow));
    assert_eq!(blurred_style.foreground, Some(Color::White));
}

/// Verifies descendant selectors match ancestor metadata in source order.
///
/// # Example Under Test
///
/// ```text
/// ancestors = [.app, .panel]
/// target = Button
/// selector = descendant([.app, .panel], Button)
/// ```
///
/// # Assertions
///
/// - The button metadata is available for stylesheet resolution.
/// - The selector matches when `.app` appears before `.panel`.
/// - The reversed ancestor order retains the default button foreground.
///
/// # Why
///
/// Descendant selector matching should honor ordered render ancestors without
/// requiring direct parent-child adjacency.
#[test]
fn descendant_selector_matches_ordered_ancestors() {
    let mut app = StyleMetadata::new(ViewType::Div);
    app.set_classes("app");
    let mut panel = StyleMetadata::new(ViewType::Block);
    panel.set_classes("panel");
    let button = button("Save");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::descendant(
            vec![StyleSelector::class("app"), StyleSelector::class("panel")],
            StyleSelector::view_type(ViewType::Button),
        ),
        TuiStyle::new().foreground(Color::Yellow),
    );

    let matched = stylesheet.resolve(
        button.style_metadata().unwrap(),
        &[app.clone(), panel.clone()],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );
    let wrong_order = stylesheet.resolve(
        button.style_metadata().unwrap(),
        &[panel, app],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(matched.foreground, Some(Color::Yellow));
    assert_eq!(wrong_order.foreground, Some(Color::White));
}
