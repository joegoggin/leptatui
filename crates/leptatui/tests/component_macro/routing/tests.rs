/// Verifies declarative routes switch component page branches.
///
/// # Example Under Test
///
/// ```text
/// <Router initial_path="/">
///   <Routes fallback=HomePage>
///     <Route path="/" view=HomePage />
///     <Route path="/counter" view=CounterPage />
///   </Routes>
/// </Router>
/// ```
///
/// # Assertions
///
/// - The initial render shows the home page branch.
/// - Re-rendering the same route does not rebuild the active page component.
/// - Updating root-owned shared state repaints the active page without rerunning root setup.
/// - A descendant component can navigate to counter and settings routes.
#[test]
fn generated_view_route_switches_pages_and_preserves_shared_state() -> leptatui::app::Result<()> {
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
fn generated_view_route_switch_rebuilds_same_type_component_with_new_props() -> leptatui::app::Result<()> {
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

/// Verifies route anchors recompute and render their pseudo-class styles.
///
/// # Example Under Test
///
/// ```text
/// <Router initial_path="/docs">
///   <A href="/" exact=true>Home</A>
///   <A href="/docs" exact=true>Docs</A>
/// </Router>
/// render, Tab, Enter, render
/// ```
///
/// # Assertions
///
/// - Inactive and active unvisited anchors render blue and underlined.
/// - The active anchor is additionally bold.
/// - The active anchor does not match an authored `.active` class rule.
/// - Activating the focused home anchor navigates to the home route.
/// - The newly active focused anchor renders magenta on dark gray after navigation.
/// - The previously active anchor returns to its inactive defaults.
#[test]
fn generated_route_link_renders_default_styles() -> leptatui::app::Result<()> {
    let mut component = MacroRouteLinkRoot::new();
    let terminal = render_component(&mut component, 12, 2)?;
    let initial = terminal.backend().buffer();
    let home = initial
        .content()
        .iter()
        .find(|cell| cell.symbol() == "H")
        .expect("home anchor cell");
    let docs = initial
        .content()
        .iter()
        .find(|cell| cell.symbol() == "D")
        .expect("docs anchor cell");
    assert_eq!(home.fg, Color::Blue);
    assert!(home.modifier.contains(Modifier::UNDERLINED));
    assert!(!home.modifier.contains(Modifier::BOLD));
    assert_eq!(docs.fg, Color::Blue);
    assert_eq!(docs.bg, Color::Reset);
    assert!(docs.modifier.contains(Modifier::BOLD));
    assert!(docs.modifier.contains(Modifier::UNDERLINED));
    assert!(!docs.modifier.contains(Modifier::REVERSED));

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Tab))?,
        AppControl::Continue,
    );
    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Enter))?,
        AppControl::Continue,
    );
    let terminal = render_component(&mut component, 12, 2)?;
    let navigated = terminal.backend().buffer();
    let home = navigated
        .content()
        .iter()
        .find(|cell| cell.symbol() == "H")
        .expect("home anchor cell");
    let docs = navigated
        .content()
        .iter()
        .find(|cell| cell.symbol() == "D")
        .expect("docs anchor cell");
    assert_eq!(home.fg, Color::Magenta);
    assert_eq!(home.bg, Color::DarkGray);
    assert!(home.modifier.contains(Modifier::BOLD));
    assert!(home.modifier.contains(Modifier::UNDERLINED));
    assert!(!home.modifier.contains(Modifier::REVERSED));
    assert_eq!(docs.fg, Color::Blue);
    assert_eq!(docs.bg, Color::Reset);
    assert!(docs.modifier.contains(Modifier::UNDERLINED));
    assert!(!docs.modifier.contains(Modifier::BOLD));

    Ok(())
}
