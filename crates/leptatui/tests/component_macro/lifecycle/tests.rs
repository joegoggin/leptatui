/// Verifies generated component setup owns persistent Leptos signal state.
///
/// # Example Under Test
///
/// ```text
/// #[component]
/// fn MacroSignalRoot() -> impl IntoView {
///     let (count, set_count) = signal(0);
///     div([dynamic(... count ...), button(... set_count ...)])
/// }
/// ```
///
/// # Assertions
///
/// - View setup runs exactly once.
/// - The first render shows the initial signal value.
/// - A key event updates the signal.
/// - A redraw shows the updated signal without rerunning setup.
#[test]
fn generated_component_setup_runs_once_and_signals_persist() -> Result<()> {
    MACRO_SIGNAL_SETUP_RUNS.store(0, Ordering::SeqCst);

    let backend = TestBackend::new(24, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut component = MacroSignalRoot::new();

    assert_eq!(MACRO_SIGNAL_SETUP_RUNS.load(Ordering::SeqCst), 1);

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = View::render(&component, &mut ctx);
    })?;
    render_result?;
    assert!(rendered_text(&terminal).contains("Count: 0"));

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('i')))?,
        AppControl::Continue
    );

    render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = View::render(&component, &mut ctx);
    })?;
    render_result?;

    assert!(rendered_text(&terminal).contains("Count: 1"));
    assert_eq!(MACRO_SIGNAL_SETUP_RUNS.load(Ordering::SeqCst), 1);

    Ok(())
}
