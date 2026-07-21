/// Verifies selector metadata remains available inside component boundaries.
///
/// # Example Under Test
///
/// ```text
/// component(MetadataRecorder)
/// text("Child").with_id("inside").with_classes("component-child")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component render call succeeds.
/// - Child metadata is recorded.
/// - The recorded id is `inside`.
/// - The recorded classes contain `component-child`.
///
/// # Why
///
/// Component boundaries should not prevent child views from carrying selector
/// metadata used by stylesheets.
#[test]
fn selector_metadata_remains_available_inside_component_boundaries() -> Result<()> {
    let seen = Rc::new(RefCell::new(None));
    let view = component(MetadataRecorder {
        seen: Rc::clone(&seen),
    });
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    let metadata = seen.borrow().clone().expect("recorded metadata");
    assert_eq!(metadata.id(), Some("inside"));
    assert_eq!(metadata.classes(), &[String::from("component-child")]);

    Ok(())
}

/// Verifies dynamic and component view boundaries render through the view tree.
///
/// # Example Under Test
///
/// ```text
/// column([dynamic(|| text("Dynamic")), component(EventExit)])
/// ```
///
/// # Assertions
///
/// - The dynamic closure is evaluated during rendering.
/// - The component boundary renders through its `View::render` method.
#[test]
fn renders_dynamic_and_component_child_views() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = column((dynamic(|| text("Dynamic")), component(EventExit)));
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Dynamic"));
    assert!(rendered.contains("Child"));

    Ok(())
}

/// Verifies view roots dispatch events through component child boundaries.
///
/// # Example Under Test
///
/// ```text
/// column([text("Static"), component(EventExit)])
///     .handle_event(Event::Resize(24, 5))
/// ```
///
/// # Assertions
///
/// - Static leaf views are skipped.
/// - Event traversal reaches the component boundary.
/// - `AppControl::Exit` short-circuits child traversal.
#[test]
fn dispatches_events_through_component_child_views() -> Result<()> {
    let mut view = column((text("Static"), component(EventExit)));

    assert_eq!(view.handle_event(Event::Resize(24, 5))?, AppControl::Exit);

    Ok(())
}

/// Verifies dynamic children are also traversed during event dispatch.
///
/// # Example Under Test
///
/// ```text
/// column([dynamic(|| component(EventCounter))])
///     .handle_event(Event::Resize(24, 5))
/// ```
///
/// # Assertions
///
/// - The dynamic closure is evaluated during event dispatch.
/// - Events reach the view produced by the dynamic closure.
#[test]
fn dispatches_events_through_dynamic_child_views() -> Result<()> {
    let count = Rc::new(Cell::new(0));
    let child_count = Rc::clone(&count);
    let mut view = column([dynamic(move || {
        component(EventCounter {
            count: Rc::clone(&child_count),
        })
    })]);

    assert_eq!(
        view.handle_event(Event::Resize(24, 5))?,
        AppControl::Continue
    );
    assert_eq!(count.get(), 1);

    Ok(())
}

/// Verifies deferred view equality stays identity-based.
///
/// # Example Under Test
///
/// ```text
/// let first = dynamic(|| text("same"));
/// let first_clone = first.clone();
/// let second = dynamic(|| text("same"));
/// ```
///
/// # Assertions
///
/// - A cloned dynamic view compares equal to its source.
/// - Separate dynamic views with identical closures do not compare equal.
#[test]
fn compares_dynamic_views_by_identity() {
    let first = dynamic(|| text("same"));
    let first_clone = first.clone();
    let second = dynamic(|| text("same"));

    assert_eq!(first, first_clone);
    assert_ne!(first, second);
}

/// Verifies editable control reconciliation retains shared runtime state.
///
/// # Example Under Test
///
/// ```text
/// reconcile(Input, previous Input with editable state)
/// reconcile(TextArea, previous TextArea with editable state)
/// ```
///
/// # Assertions
///
/// - Matching editable variants preserve focus.
/// - Matching editable variants preserve cursor, scroll, mode, selection, yank,
///   undo, and redo state.
#[test]
fn reconciliation_retains_editable_state_for_matching_controls() {
    let retained_input_state = editable_state_fixture();
    let mut previous_input = editable_input("old").with_focus(true);
    *previous_input.editable_state_mut() = retained_input_state.clone();
    let mut next_input = editable_input("new");

    leptatui::__private::__reconcile_view(&mut next_input, &previous_input);

    assert!(next_input.style_metadata().unwrap().is_focused());
    assert_eq!(editable_state(&next_input), &retained_input_state);

    let retained_text_area_state = editable_state_fixture();
    let mut previous_text_area = editable_text_area("old notes").with_focus(true);
    *previous_text_area.editable_state_mut() = retained_text_area_state.clone();
    let mut next_text_area = editable_text_area("new notes");

    leptatui::__private::__reconcile_view(&mut next_text_area, &previous_text_area);

    assert!(next_text_area.style_metadata().unwrap().is_focused());
    assert_eq!(editable_state(&next_text_area), &retained_text_area_state);
}

/// Verifies editable control reconciliation does not leak state across unrelated views.
///
/// # Example Under Test
///
/// ```text
/// reconcile(TextArea, previous Input with editable state)
/// reconcile(Input, previous TextArea with editable state)
/// reconcile(Button, previous Input with editable state)
/// ```
///
/// # Assertions
///
/// - Mismatched editable variants do not preserve focus.
/// - Mismatched editable variants keep their fresh editable state.
/// - Buttons do not inherit focus from previous editable controls.
#[test]
fn reconciliation_does_not_leak_editable_state_to_unrelated_views() {
    let retained_state = editable_state_fixture();
    let mut previous_input = editable_input("old").with_focus(true);
    *previous_input.editable_state_mut() = retained_state.clone();

    let mut next_text_area = editable_text_area("new notes");
    let fresh_text_area = editable_text_area("new notes");
    leptatui::__private::__reconcile_view(&mut next_text_area, &previous_input);

    assert!(!next_text_area.style_metadata().unwrap().is_focused());
    assert_eq!(
        editable_state(&next_text_area),
        editable_state(&fresh_text_area)
    );
    assert_ne!(editable_state(&next_text_area), &retained_state);

    let mut previous_text_area = editable_text_area("old notes").with_focus(true);
    *previous_text_area.editable_state_mut() = retained_state.clone();

    let mut next_input = editable_input("new");
    let fresh_input = editable_input("new");
    leptatui::__private::__reconcile_view(&mut next_input, &previous_text_area);

    assert!(!next_input.style_metadata().unwrap().is_focused());
    assert_eq!(editable_state(&next_input), editable_state(&fresh_input));
    assert_ne!(editable_state(&next_input), &retained_state);

    let mut next_button = button("Submit");
    leptatui::__private::__reconcile_view(&mut next_button, &previous_input);

    assert!(!next_button.style_metadata().unwrap().is_focused());
}

/// Verifies dynamic reconciliation replaces newly produced nested dynamic boundaries.
///
/// # Example Under Test
///
/// ```text
/// dynamic(|| dynamic(|| text(route_label)))
/// ```
///
/// # Assertions
///
/// - The first render shows the initial inner dynamic closure output.
/// - Updating the outer dynamic state replaces the previous inner dynamic closure.
#[test]
fn dynamic_reconciliation_replaces_new_nested_dynamic_boundaries() -> Result<()> {
    let label = Rc::new(Cell::new("Home"));
    let dynamic_label = Rc::clone(&label);
    let view = dynamic(move || {
        let current = dynamic_label.get();
        dynamic(move || text(current))
    });
    let mut terminal = Terminal::new(TestBackend::new(16, 1))?;

    draw_view(&mut terminal, &view)?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Home"), "rendered text: {rendered:?}");

    label.set("Counter");
    draw_view(&mut terminal, &view)?;
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Counter"), "rendered text: {rendered:?}");

    Ok(())
}
