/// Verifies a bare `stylesheet!` statement registers with its component.
///
/// # Assertions
///
/// - The component's text receives the foreground and background from the
///   local stylesheet.
#[test]
fn generated_component_stylesheet_styles_own_views() -> Result<()> {
    let mut component = MacroStyledText::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(
        rendered_cell_colors(&terminal, "S"),
        (Color::Yellow, Color::Blue)
    );

    Ok(())
}

/// Verifies component styles do not leak into sibling component subtrees.
///
/// # Assertions
///
/// - The styled sibling receives the shared class style.
/// - The plain sibling has the same class but keeps default colors.
#[test]
fn generated_component_stylesheets_do_not_leak_to_siblings() -> Result<()> {
    let mut component = MacroSiblingStyleRoot::new();
    let terminal = render_component(&mut component, 24, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "S").0, Color::Yellow);
    assert_eq!(rendered_cell_colors(&terminal, "P").0, Color::Reset);

    Ok(())
}

/// Verifies parent component styles apply through child component boundaries.
///
/// # Assertions
///
/// - A child component's text receives the parent component stylesheet.
#[test]
fn generated_component_stylesheets_apply_to_child_component_subtrees() -> Result<()> {
    let mut component = MacroParentStylesChild::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "P").0, Color::Green);

    Ok(())
}

/// Verifies equal-specificity child component styles win by source order.
///
/// # Assertions
///
/// - A child text rule overrides an equal-specificity parent text rule.
#[test]
fn generated_equal_specificity_child_stylesheet_wins_by_source_order() -> Result<()> {
    let mut component = MacroParentWithChildOverride::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "O").0, Color::Yellow);

    Ok(())
}

/// Verifies parent component specificity participates in the CSS cascade.
///
/// # Assertions
///
/// - A parent class rule overrides a lower-specificity child text rule.
#[test]
fn generated_higher_specificity_parent_stylesheet_overrides_child_stylesheet() -> Result<()> {
    let mut component = MacroParentSpecificityBeatsChild::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "S").0, Color::Green);

    Ok(())
}

/// Verifies component stylesheets resolve against component-provided themes.
///
/// # Assertions
///
/// - A `theme_color(...)` declaration resolves from context during render.
#[test]
fn generated_component_stylesheet_resolves_theme_context() -> Result<()> {
    let mut component = MacroThemedStylesheet::new();
    let terminal = render_component(&mut component, 16, 3)?;

    assert_eq!(rendered_cell_colors(&terminal, "T").0, Color::LightCyan);

    Ok(())
}
