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
fn generated_component_props_render() -> leptatui::app::Result<()> {
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

/// Verifies vector child expressions splice views directly into a container.
///
/// # Assertions
///
/// - A `Children` callback contributes each returned view in source order.
/// - A general homogeneous vector expression is flattened the same way.
/// - An empty vector contributes no retained wrapper or child.
#[test]
fn vector_child_expressions_splice_directly_into_containers() {
    let children: Children = Box::new(|| {
        vec![
            text("First").into_view(),
            text("Second").into_view(),
        ]
    });
    let from_callback = view! { <Div>{children()}</Div> };

    assert_eq!(from_callback.children().len(), 2);
    assert_eq!(
        from_callback.children()[0]
            .downcast_ref::<leptatui::TextView>()
            .expect("expected first text child")
            .content()
            .to_string(),
        "First"
    );
    assert_eq!(
        from_callback.children()[1]
            .downcast_ref::<leptatui::TextView>()
            .expect("expected second text child")
            .content()
            .to_string(),
        "Second"
    );

    let values = vec![text("Third"), text("Fourth")];
    let from_vector = view! { <Div>{values}</Div> };
    assert_eq!(from_vector.children().len(), 2);

    let empty = Vec::<AnyView>::new();
    let from_empty = view! { <Div>{empty}</Div> };
    assert!(from_empty.children().is_empty());
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
fn generated_component_min_height_tracks_responsive_internal_layout() -> leptatui::app::Result<()> {
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
fn generated_component_scroll_keys_cross_component_boundaries() -> leptatui::app::Result<()> {
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

/// Verifies generated component roots preserve retained box geometry while scrolling.
///
/// # Example Under Test
///
/// ```text
/// MacroBorderedScrollableRoot
/// terminal = 12x4
/// PageDown
/// ```
///
/// # Assertions
///
/// - Page Down reaches the overflowing component root.
/// - The scrolled content displays the final list item.
/// - The top and bottom border rows remain intact.
///
/// # Why
///
/// Component rendering must retain the root view's border, padding, content,
/// viewport, and clip rectangles instead of replacing them with one identity
/// rectangle.
#[test]
fn generated_component_roots_preserve_scrolling_box_geometry() -> leptatui::app::Result<()> {
    let mut component = MacroBorderedScrollableRoot::new();
    render_component(&mut component, 12, 4)?;

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::PageDown))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 12, 4)?;
    let lines = rendered_lines(&terminal);
    assert!(rendered_text(&terminal).contains("Six"));
    assert_eq!(lines[0], "┌──────────┐");
    assert_eq!(lines[3], "└──────────┘");

    Ok(())
}

/// Verifies off-screen generated components release their mouse hit areas.
///
/// # Example Under Test
///
/// ```text
/// MacroScrolledMouseRoot(Hidden, Visible)
/// PageDown
/// MouseMoved(1, 1)
/// ```
///
/// # Assertions
///
/// - The initial render displays the hidden control.
/// - Scrolling replaces it with the visible control.
/// - Pointer movement focuses the visible control rather than the stale one.
#[test]
fn offscreen_generated_component_hit_areas_are_cleared() -> leptatui::app::Result<()> {
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

/// Verifies Markdown history keys cross generated and stored component boundaries.
///
/// # Example Under Test
///
/// ```text
/// MacroMarkdownHistoryBoundary(MacroMarkdownHistoryProbe)
/// Shift+H
/// Shift+L
/// ```
///
/// # Assertions
///
/// - Shift+H reaches a probe inside a generated component root.
/// - Shift+L reaches a probe through both generated and stored component boundaries.
#[test]
fn markdown_history_keys_cross_component_boundaries() -> leptatui::app::Result<()> {
    let direct_direction = Rc::new(Cell::new(None));
    let mut direct = MacroMarkdownHistoryBoundary::with_props(
        MacroMarkdownHistoryBoundaryProps::builder()
            .probe(
                MacroMarkdownHistoryProbe {
                    direction: Rc::clone(&direct_direction),
                }
                .into_view(),
            )
            .build(),
    );
    View::handle_event(
        &mut direct,
        Event::Key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT)),
    )?;
    assert_eq!(direct_direction.get(), Some(true));

    let nested_direction = Rc::new(Cell::new(None));
    let nested = MacroMarkdownHistoryBoundary::with_props(
        MacroMarkdownHistoryBoundaryProps::builder()
            .probe(
                MacroMarkdownHistoryProbe {
                    direction: Rc::clone(&nested_direction),
                }
                .into_view(),
            )
            .build(),
    );
    let mut boundary = component(nested);
    boundary.handle_event(Event::Key(KeyEvent::new(
        KeyCode::Char('L'),
        KeyModifiers::SHIFT,
    )))?;
    assert_eq!(nested_direction.get(), Some(false));

    Ok(())
}
