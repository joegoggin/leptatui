/// Verifies Leptos owner context remains available through Leptatui lookup.
///
/// # Example Under Test
///
/// ```text
/// Owner::new().with(|| {
///     leptos::context::provide_context(String::from("from leptos"));
///     use_context::<String>()
/// })
/// ```
///
/// # Assertions
///
/// - `use_context` finds the Leptos-provided string.
/// - `expect_context` returns the Leptos-provided string.
///
/// # Why
///
/// Leptatui context helpers should not break existing Leptos owner context.
#[test]
fn leptos_owner_context_fallback_still_works() {
    Owner::new().with(|| {
        leptos::context::provide_context(String::from("from leptos"));

        assert_eq!(use_context::<String>().as_deref(), Some("from leptos"));
        assert_eq!(expect_context::<String>(), "from leptos");
    });
}

/// Verifies context follows component subtree ancestry during rendering.
///
/// # Example Under Test
///
/// ```text
/// outer provider
///   consumer -> outer
///   inner provider
///     consumer -> inner
///   consumer -> outer
/// ```
///
/// # Assertions
///
/// - The first descendant sees the outer provider value.
/// - The inner descendant sees the inner provider value.
/// - The sibling after the inner provider sees the restored outer value.
#[test]
fn component_context_is_scoped_to_render_subtrees() -> Result<()> {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let backend = TestBackend::new(24, 6);
    let mut terminal = Terminal::new(backend)?;
    let view = component(LabelProvider {
        value: ScopeLabel("outer"),
        child: div([
            component(LabelConsumer::new(Rc::clone(&observed))),
            component(LabelProvider {
                value: ScopeLabel("inner"),
                child: component(LabelConsumer::new(Rc::clone(&observed))),
            }),
            component(LabelConsumer::new(Rc::clone(&observed))),
        ])
        .into_view(),
    });
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        observed.borrow().as_slice(),
        [
            Some(ScopeLabel("outer")),
            Some(ScopeLabel("inner")),
            Some(ScopeLabel("outer")),
        ]
    );

    Ok(())
}

/// Verifies provider ancestry is available during descendant event handling.
///
/// # Example Under Test
///
/// ```text
/// render provider -> stores ScopeLabel("event")
/// dispatch event through same provider subtree
/// child event handler reads ScopeLabel("event")
/// ```
///
/// # Assertions
///
/// - The initial render succeeds.
/// - Event traversal continues.
/// - The child event handler sees the provider value from the latest render.
#[test]
fn component_context_is_available_during_descendant_events() -> Result<()> {
    let observed = Rc::new(RefCell::new(None));
    let backend = TestBackend::new(24, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = component(EventLabelProvider {
        value: ScopeLabel("event"),
        child: component(EventLabelConsumer {
            observed: Rc::clone(&observed),
        }),
    });
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        view.handle_event(crossterm::event::Event::Resize(24, 4))?,
        AppControl::Continue
    );
    assert_eq!(*observed.borrow(), Some(ScopeLabel("event")));

    Ok(())
}
