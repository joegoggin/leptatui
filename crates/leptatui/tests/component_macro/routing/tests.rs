/// Verifies route state can drive dynamic `view!` page branches.
///
/// # Example Under Test
///
/// ```text
/// provide_route(Home)
/// <Div>
///   <RouteKeyNav />
///   {move || match route.get_untracked() { Home => <HomePage />, ... }}
/// </Div>
/// ```
///
/// # Assertions
///
/// - The initial render shows the home page branch.
/// - Re-rendering the same route does not rebuild the active page component.
/// - Updating root-owned shared state repaints the active page without rerunning root setup.
/// - A descendant component can navigate to counter and settings routes.
#[test]
fn generated_view_route_switches_pages_and_preserves_shared_state() -> Result<()> {
    MACRO_ROUTE_ROOT_SETUP_RUNS.store(0, Ordering::SeqCst);
    MACRO_ROUTE_HOME_SETUP_RUNS.store(0, Ordering::SeqCst);
    MACRO_ROUTE_COUNTER_SETUP_RUNS.store(0, Ordering::SeqCst);
    MACRO_ROUTE_SETTINGS_SETUP_RUNS.store(0, Ordering::SeqCst);

    let mut component = MacroRouteSwitchRoot::new();

    assert_eq!(MACRO_ROUTE_ROOT_SETUP_RUNS.load(Ordering::SeqCst), 1);

    let terminal = render_component(&mut component, 32, 4)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Home 0"), "rendered text: {text:?}");
    assert_eq!(MACRO_ROUTE_HOME_SETUP_RUNS.load(Ordering::SeqCst), 1);

    let terminal = render_component(&mut component, 32, 4)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Home 0"), "rendered text: {text:?}");
    assert_eq!(MACRO_ROUTE_HOME_SETUP_RUNS.load(Ordering::SeqCst), 1);

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('i')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 32, 4)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Home 1"), "rendered text: {text:?}");
    assert_eq!(MACRO_ROUTE_ROOT_SETUP_RUNS.load(Ordering::SeqCst), 1);
    assert_eq!(MACRO_ROUTE_HOME_SETUP_RUNS.load(Ordering::SeqCst), 1);

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('c')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 32, 4)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Counter 1"), "rendered text: {text:?}");
    assert_eq!(MACRO_ROUTE_COUNTER_SETUP_RUNS.load(Ordering::SeqCst), 1);

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('s')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 32, 4)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Settings 1"), "rendered text: {text:?}");
    assert_eq!(MACRO_ROUTE_SETTINGS_SETUP_RUNS.load(Ordering::SeqCst), 1);

    Ok(())
}

/// Verifies route branches using the same component type do not keep stale props.
///
/// # Example Under Test
///
/// ```text
/// match route {
///   Home => <NamedPage label="Home" />,
///   Counter => <NamedPage label="Counter" />,
/// }
/// ```
///
/// # Assertions
///
/// - The initial route renders the first prop value.
/// - Navigating to another branch with the same component type renders the new prop value.
#[test]
fn generated_view_route_switch_rebuilds_same_type_component_with_new_props() -> Result<()> {
    let mut component = MacroRoutePropSwitchRoot::new();

    let terminal = render_component(&mut component, 32, 4)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Home"), "rendered text: {text:?}");

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('c')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 32, 4)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Counter"), "rendered text: {text:?}");
    assert!(!text.contains("Home"), "rendered text: {text:?}");

    Ok(())
}
