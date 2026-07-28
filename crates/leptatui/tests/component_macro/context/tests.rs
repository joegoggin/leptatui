/// Verifies generated component providers remain visible to descendants.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroContextProvider() -> impl IntoView {
///     provide_context(MacroLabel("macro"));
///     component(MacroContextConsumer)
/// }
/// ```
///
/// # Assertions
///
/// - The generated provider renders successfully.
/// - The descendant component reads the context value provided during setup.
///
/// # Why
///
/// Generated component bodies run once under a stored Leptos owner whose
/// context remains active while rendering the returned view tree.
#[test]
fn generated_component_providers_are_visible_to_descendants() -> Result<()> {
    let backend = TestBackend::new(16, 3);
    let mut terminal = Terminal::new(backend)?;
    let component = MacroContextProvider::new();

    for _ in 0..2 {
        *MACRO_CONTEXT_OBSERVED
            .lock()
            .expect("context observation lock should be available") = None;

        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            render_result = View::render(&component, &mut ctx);
        })?;
        render_result?;

        assert_eq!(
            *MACRO_CONTEXT_OBSERVED
                .lock()
                .expect("context observation lock should be available"),
            Some(MacroLabel("macro"))
        );
    }

    Ok(())
}

/// Verifies lazy generated component providers retain context across renders.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn LazyRoot() -> impl IntoView {
///     view! { <ContextProvider /> }
/// }
/// ```
///
/// # Assertions
///
/// - The lazy provider renders successfully twice.
/// - Its descendant reads the setup-time context during both renders.
///
/// # Why
///
/// Lazy component setup may run inside a temporary render scope, but provided
/// values must remain attached to the component owner after that scope exits.
#[test]
fn lazy_generated_component_provider_context_survives_multiple_renders() -> Result<()> {
    let backend = TestBackend::new(16, 3);
    let mut terminal = Terminal::new(backend)?;
    let component = MacroLazyContextRoot::new();

    for _ in 0..2 {
        *MACRO_CONTEXT_OBSERVED
            .lock()
            .expect("context observation lock should be available") = None;

        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            render_result = View::render(&component, &mut ctx);
        })?;
        render_result?;

        assert_eq!(
            *MACRO_CONTEXT_OBSERVED
                .lock()
                .expect("context observation lock should be available"),
            Some(MacroLabel("macro"))
        );
    }

    Ok(())
}
