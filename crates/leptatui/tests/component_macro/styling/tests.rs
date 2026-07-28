/// Verifies a bare `stylesheet!` statement registers with its component.
///
/// # Example Under Test
///
/// ```text
/// MacroStyledText::new()
/// local Text rule: yellow on blue
/// ```
///
/// # Assertions
///
/// - The component's text receives the foreground and background from the
///   local stylesheet.
#[test]
fn generated_component_stylesheet_styles_own_views() -> leptatui::app::Result<()> {
    let mut component = MacroStyledText::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(
        rendered_cell_colors(&terminal, "S"),
        (Color::Yellow, Color::Blue)
    );

    Ok(())
}

/// Verifies layout traversal enters generated component stylesheet scopes.
///
/// # Example Under Test
///
/// ```text
/// MacroHiddenLayoutChild
/// .hidden { display: none }
/// ```
///
/// # Assertions
///
/// - The generated component renders successfully.
/// - Its locally hidden text is absent from the terminal.
#[test]
fn generated_component_layout_resolves_local_display_none() -> leptatui::app::Result<()> {
    let mut component = MacroHiddenLayoutChild::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert!(!rendered_text(&terminal).contains("Hidden"));

    Ok(())
}

/// Verifies component styles do not leak into sibling component subtrees.
///
/// # Example Under Test
///
/// ```text
/// MacroSiblingStyleRoot(MacroStyledSibling, MacroPlainSibling)
/// ```
///
/// # Assertions
///
/// - The styled sibling receives the shared class style.
/// - The plain sibling has the same class but keeps default colors.
#[test]
fn generated_component_stylesheets_do_not_leak_to_siblings() -> leptatui::app::Result<()> {
    let mut component = MacroSiblingStyleRoot::new();
    let terminal = render_component(&mut component, 24, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "S").0, Color::Yellow);
    assert_eq!(rendered_cell_colors(&terminal, "P").0, Color::Reset);

    Ok(())
}

/// Verifies parent component styles apply through child component boundaries.
///
/// # Example Under Test
///
/// ```text
/// MacroParentStylesChild -> MacroPlainSibling
/// ```
///
/// # Assertions
///
/// - A child component's text receives the parent component stylesheet.
#[test]
fn generated_component_stylesheets_apply_to_child_component_subtrees() -> leptatui::app::Result<()> {
    let mut component = MacroParentStylesChild::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "P").0, Color::Green);

    Ok(())
}

/// Verifies equal-specificity child component styles win by source order.
///
/// # Example Under Test
///
/// ```text
/// MacroParentWithChildOverride -> MacroChildStyleOverride
/// parent Text rule, then child Text rule
/// ```
///
/// # Assertions
///
/// - A child text rule overrides an equal-specificity parent text rule.
#[test]
fn generated_equal_specificity_child_stylesheet_wins_by_source_order() -> leptatui::app::Result<()> {
    let mut component = MacroParentWithChildOverride::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "O").0, Color::Yellow);

    Ok(())
}

/// Verifies parent component specificity participates in the CSS cascade.
///
/// # Example Under Test
///
/// ```text
/// MacroParentSpecificityBeatsChild -> MacroChildLowerSpecificity
/// parent class rule, child Text rule
/// ```
///
/// # Assertions
///
/// - A parent class rule overrides a lower-specificity child text rule.
#[test]
fn generated_higher_specificity_parent_stylesheet_overrides_child_stylesheet() -> leptatui::app::Result<()> {
    let mut component = MacroParentSpecificityBeatsChild::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "S").0, Color::Green);

    Ok(())
}

/// Verifies component stylesheets resolve against component-provided themes.
///
/// # Example Under Test
///
/// ```text
/// MacroThemedStylesheet::new()
/// theme_color(...) -> LightCyan
/// ```
///
/// # Assertions
///
/// - A `theme_color(...)` declaration resolves from context during render.
#[test]
fn generated_component_stylesheet_resolves_theme_context() -> leptatui::app::Result<()> {
    let mut component = MacroThemedStylesheet::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "T").0, Color::LightCyan);

    Ok(())
}
