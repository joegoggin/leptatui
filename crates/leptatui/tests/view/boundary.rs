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
fn selector_metadata_remains_available_inside_component_boundaries() -> leptatui::app::Result<()> {
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
/// div([dynamic(|| text("Dynamic")), component(EventExit)])
/// ```
///
/// # Assertions
///
/// - The dynamic closure is evaluated during rendering.
/// - The component boundary renders through its `View::render` method.
#[test]
fn renders_dynamic_and_component_child_views() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = div((dynamic(|| text("Dynamic")), component(EventExit)));
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
/// div([text("Static"), component(EventExit)])
///     .handle_event(Event::Resize(24, 5))
/// ```
///
/// # Assertions
///
/// - Static leaf views are skipped.
/// - Event traversal reaches the component boundary.
/// - `AppControl::Exit` short-circuits child traversal.
#[test]
fn dispatches_events_through_component_child_views() -> leptatui::app::Result<()> {
    let mut view = div((text("Static"), component(EventExit)));

    assert_eq!(view.handle_event(Event::Resize(24, 5))?, AppControl::Exit);

    Ok(())
}

/// Verifies dynamic children are also traversed during event dispatch.
///
/// # Example Under Test
///
/// ```text
/// div([dynamic(|| component(EventCounter))])
///     .handle_event(Event::Resize(24, 5))
/// ```
///
/// # Assertions
///
/// - The dynamic closure is evaluated during event dispatch.
/// - Events reach the view produced by the dynamic closure.
#[test]
fn dispatches_events_through_dynamic_child_views() -> leptatui::app::Result<()> {
    let count = Rc::new(Cell::new(0));
    let child_count = Rc::clone(&count);
    let mut view = div([dynamic(move || {
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

/// Verifies keyed dynamic children rebuild only after key changes.
///
/// # Example Under Test
///
/// ```text
/// key = 0
/// draw
/// draw
/// key = 1
/// draw
/// ```
///
/// # Assertions
///
/// - The first draw constructs and renders the initial child.
/// - A second draw with the same key reuses the existing child.
/// - Changing the key constructs and renders one replacement child.
///
/// # Why
///
/// Expensive view factories such as Markdown parsing should not rerun for
/// unrelated redraws.
#[test]
fn keyed_views_rebuild_children_only_when_keys_change() -> leptatui::app::Result<()> {
    Owner::new().with(|| {
        let key = RwSignal::new(0_u8);
        let builds = Rc::new(Cell::new(0_u8));
        let child_builds = Rc::clone(&builds);
        let view = keyed(
            move || key.get(),
            move || {
                child_builds.set(child_builds.get().saturating_add(1));
                text(format!("Key {}", key.get_untracked()))
            },
        );
        let mut terminal = Terminal::new(TestBackend::new(24, 5))?;

        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 1);
        assert!(rendered_text(&terminal).contains("Key 0"));

        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 1);

        key.set(1);
        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 2);
        assert!(rendered_text(&terminal).contains("Key 1"));

        Ok(())
    })
}

/// Verifies dynamic views rebuild only after a tracked dependency changes.
///
/// # Example Under Test
///
/// ```text
/// dynamic(move || text(format!("Count {}", count.get())))
/// ```
///
/// # Assertions
///
/// - The first draw builds the child once.
/// - An unrelated second draw reuses the retained child.
/// - Updating the tracked signal rebuilds the child once.
#[test]
fn dynamic_views_rebuild_after_tracked_signal_changes() -> leptatui::app::Result<()> {
    Owner::new().with(|| {
        let count = RwSignal::new(0_u8);
        let builds = Rc::new(Cell::new(0_u8));
        let child_builds = Rc::clone(&builds);
        let view = dynamic(move || {
            child_builds.set(child_builds.get().saturating_add(1));
            text(format!("Count {}", count.get()))
        });
        let mut terminal = Terminal::new(TestBackend::new(24, 5))?;

        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 1);
        assert!(rendered_text(&terminal).contains("Count 0"));

        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 1);

        count.set(1);
        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 2);
        assert!(rendered_text(&terminal).contains("Count 1"));

        Ok(())
    })
}

/// Verifies a dynamic view respects memo change detection.
///
/// # Example Under Test
///
/// ```text
/// parity = Memo::new(move |_| count.get() % 2)
/// dynamic(move || text(parity.get().to_string()))
/// ```
///
/// # Assertions
///
/// - An upstream change that preserves the memo value does not rebuild.
/// - An upstream change that alters the memo value rebuilds once.
#[test]
fn dynamic_views_rebuild_only_when_memo_values_change() -> leptatui::app::Result<()> {
    Owner::new().with(|| {
        let count = RwSignal::new(0_u8);
        let parity = Memo::new(move |_| count.get() % 2);
        let builds = Rc::new(Cell::new(0_u8));
        let child_builds = Rc::clone(&builds);
        let view = dynamic(move || {
            child_builds.set(child_builds.get().saturating_add(1));
            text(parity.get().to_string())
        });
        let mut terminal = Terminal::new(TestBackend::new(24, 5))?;

        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 1);

        count.set(2);
        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 1);

        count.set(3);
        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 2);
        assert!(rendered_text(&terminal).contains('1'));

        Ok(())
    })
}

/// Verifies untracked reads do not invalidate a dynamic view.
///
/// # Example Under Test
///
/// ```text
/// dynamic(move || text(format!("Count {}", count.get_untracked())))
/// ```
///
/// # Assertions
///
/// - The first draw builds the child once.
/// - Updating the untracked signal does not rebuild the child.
/// - The retained child continues to show its initial value.
#[test]
fn dynamic_views_ignore_untracked_signal_changes() -> leptatui::app::Result<()> {
    Owner::new().with(|| {
        let count = RwSignal::new(0_u8);
        let builds = Rc::new(Cell::new(0_u8));
        let child_builds = Rc::clone(&builds);
        let view = dynamic(move || {
            child_builds.set(child_builds.get().saturating_add(1));
            text(format!("Count {}", count.get_untracked()))
        });
        let mut terminal = Terminal::new(TestBackend::new(24, 5))?;

        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 1);
        assert!(rendered_text(&terminal).contains("Count 0"));

        count.set(1);
        draw_view(&mut terminal, &view)?;
        assert_eq!(builds.get(), 1);
        assert!(rendered_text(&terminal).contains("Count 0"));

        Ok(())
    })
}

/// Verifies `view!` text closures track signal reads and retain attributes.
///
/// # Example Under Test
///
/// ```text
/// <Text style=yellow>{move || label.get()}</Text>
/// ```
///
/// # Assertions
///
/// - The initial signal value is rendered.
/// - Updating the signal changes the rendered text.
/// - The inline style is applied to the rebuilt text view.
#[test]
fn view_macro_text_closures_render_reactively() -> leptatui::app::Result<()> {
    Owner::new().with(|| {
        let label = RwSignal::new(String::from("Idle"));
        let view = view! {
            <Text style=TuiStyle::new().foreground(Color::Yellow)>
                {move || label.get()}
            </Text>
        };
        let mut terminal = Terminal::new(TestBackend::new(24, 5))?;

        draw_view(&mut terminal, &view)?;
        assert!(rendered_text(&terminal).contains("Idle"));
        let cell = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .find(|cell| cell.symbol() == "I")
            .expect("rendered I cell");
        assert_eq!(cell.fg, Color::Yellow);

        label.set(String::from("Saved"));
        draw_view(&mut terminal, &view)?;
        assert!(rendered_text(&terminal).contains("Saved"));

        Ok(())
    })
}

/// Verifies `view!` treats a direct signal child as a tracked value read.
///
/// # Example Under Test
///
/// ```text
/// <Text>{label}</Text>
/// ```
///
/// # Assertions
///
/// - The initial signal value is rendered without an explicit `.get()` call.
/// - Updating the signal changes the rendered text.
#[test]
fn view_macro_direct_text_signals_render_reactively() -> leptatui::app::Result<()> {
    Owner::new().with(|| {
        let label = RwSignal::new(String::from("Idle"));
        let view = view! { <Text>{label}</Text> };
        let mut terminal = Terminal::new(TestBackend::new(24, 5))?;

        draw_view(&mut terminal, &view)?;
        assert!(rendered_text(&terminal).contains("Idle"));

        label.set(String::from("Saved"));
        draw_view(&mut terminal, &view)?;
        assert!(rendered_text(&terminal).contains("Saved"));

        Ok(())
    })
}

/// Verifies direct signal children work in ordinary container positions.
///
/// # Example Under Test
///
/// ```text
/// <Div>{label}</Div>
/// ```
///
/// # Assertions
///
/// - The initial signal value becomes a text child.
/// - Updating the signal changes the retained container's rendered content.
#[test]
fn view_macro_direct_container_signals_render_reactively() -> leptatui::app::Result<()> {
    Owner::new().with(|| {
        let label = RwSignal::new(String::from("Idle"));
        let view = view! { <Div>{label}</Div> };
        let mut terminal = Terminal::new(TestBackend::new(24, 5))?;

        draw_view(&mut terminal, &view)?;
        assert!(rendered_text(&terminal).contains("Idle"));

        label.set(String::from("Saved"));
        draw_view(&mut terminal, &view)?;
        assert!(rendered_text(&terminal).contains("Saved"));

        Ok(())
    })
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

/// Verifies reconciliation distinguishes semantic variants sharing one Rust type.
///
/// # Example Under Test
///
/// ```text
/// reconcile(Div block, previous Div flex)
/// reconcile(UnorderedList, previous OrderedList)
/// reconcile(TableBody, previous TableHead)
/// reconcile(H2, previous H1)
/// ```
///
/// # Assertions
///
/// - A `Div` remains compatible when its layout styles change.
/// - Focused descendant state crosses a compatible layout style change.
/// - Different list, table-section, and heading variants remain incompatible.
#[test]
fn reconciliation_preserves_div_state_across_layout_styles() {
    assert!(div(()).can_reconcile_from(
        &div(()).with_inline_style(TuiStyle::new().display(Display::Flex))
    ));
    assert!(!unordered_list(()).can_reconcile_from(&ordered_list(())));
    assert!(!table_body(()).can_reconcile_from(&table_head(())));
    assert!(!h2("Heading").can_reconcile_from(&h1("Heading")));

    let previous = div([button("Action").with_focus(true)])
        .with_inline_style(TuiStyle::new().display(Display::Flex));
    let mut next = div([button("Action")]);
    leptatui::__private::__reconcile_view(&mut next, &previous);

    let button = next.children()[0]
        .downcast_ref::<ButtonView>()
        .expect("expected button view");
    assert!(button.metadata().is_focused());
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
fn dynamic_reconciliation_replaces_new_nested_dynamic_boundaries() -> leptatui::app::Result<()> {
    Owner::new().with(|| {
        let label = RwSignal::new("Home");
        let view = dynamic(move || {
            let current = label.get();
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
    })
}
