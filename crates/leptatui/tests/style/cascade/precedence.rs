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
