/// Verifies generated props are available while the component tree is built.
///
/// # Example Under Test
///
/// ```text
/// MacroPropPanel(title = "Panel", children = [MacroPropLabel("Child")])
/// ```
///
/// # Assertions
///
/// - A required `into` prop renders as text.
/// - Nested children passed through a `Children` prop render inside the panel.
#[test]
fn generated_component_props_render() -> Result<()> {
    let mut component = MacroPropPanel::with_props(
        MacroPropPanelProps::builder()
            .title("Panel")
            .children(Box::new(|| {
                vec![
                    MacroPropLabel::with_props(
                        MacroPropLabelProps::builder().label("Child").build(),
                    )
                    .into_view(),
                ]
            }))
            .build(),
    );
    let terminal = render_component(&mut component, 24, 6)?;
    let text = rendered_text(&terminal);

    assert!(text.contains("Panel"), "rendered text: {text:?}");
    assert!(text.contains("Child"), "rendered text: {text:?}");

    Ok(())
}

/// Verifies generated component boundaries report responsive internal height.
///
/// # Example Under Test
///
/// ```text
/// MacroResponsiveCaseRoot::new()
/// viewport width = 40, height = 3
/// ```
///
/// # Assertions
///
/// - The root content renders successfully.
/// - The responsive class rule wins over the lower-specificity type rule.
/// - Content below the responsive row remains visible within the measured height.
#[test]
fn generated_component_min_height_tracks_responsive_internal_layout() -> Result<()> {
    let mut component = MacroResponsiveCaseRoot::new();
    let terminal = render_component(&mut component, 40, 3)?;
    let text = rendered_text(&terminal);

    assert!(text.contains("Intro"), "rendered text: {text:?}");
    assert!(text.contains("type < class"), "rendered text: {text:?}");
    assert!(text.contains("Sample"), "rendered text: {text:?}");

    Ok(())
}

/// Verifies default scroll keys cross generated component boundaries.
///
/// # Example Under Test
///
/// ```text
/// Row(<MacroScrollableList />)
/// PageDown, gg, G
/// ```
///
/// # Assertions
///
/// - The initial render shows the top of the child component list.
/// - PageDown scrolls the child component's overflowing column.
/// - `gg` returns the child component's overflowing column to the top.
/// - `G` scrolls the child component's overflowing column to the bottom.
#[test]
fn generated_component_scroll_keys_cross_component_boundaries() -> Result<()> {
    let mut component = MacroScrollableBoundaryRoot::new();
    let terminal = render_component(&mut component, 12, 3)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("One"), "rendered text: {text:?}");
    assert!(!text.contains("Six"), "rendered text: {text:?}");

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::PageDown))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 12, 3)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Six"), "rendered text: {text:?}");

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('g')))?,
        AppControl::Continue
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('g')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 12, 3)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("One"), "rendered text: {text:?}");
    assert!(!text.contains("Six"), "rendered text: {text:?}");

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('G')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 12, 3)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Six"), "rendered text: {text:?}");

    Ok(())
}

/// Verifies off-screen generated components release their mouse hit areas.
#[test]
fn offscreen_generated_component_hit_areas_are_cleared() -> Result<()> {
    let mut component = MacroScrolledMouseRoot::new();
    let terminal = render_component(&mut component, 12, 3)?;
    assert!(rendered_text(&terminal).contains("Hidden"));

    View::handle_event(&mut component, key(KeyCode::PageDown))?;
    let terminal = render_component(&mut component, 12, 3)?;
    let text = rendered_text(&terminal);
    assert!(!text.contains("Hidden"), "rendered text: {text:?}");
    assert!(text.contains("Visible"), "rendered text: {text:?}");

    View::handle_event(
        &mut component,
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        }),
    )?;
    let mut index = 0;
    assert_eq!(
        View::__focused_index_inner(&component, &mut index),
        Some(1)
    );

    Ok(())
}
