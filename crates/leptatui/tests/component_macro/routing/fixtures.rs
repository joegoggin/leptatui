/// Root component that renders page branches from route state.
#[component]
fn MacroRouteSwitchRoot() -> impl leptatui::IntoView {
    MACRO_ROUTE_ROOT_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);

    let (shared_count, set_shared_count) = signal(0);
    provide_context(MacroSharedCount(shared_count));
    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('i') {
            set_shared_count.update(|count| *count += 1);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    view! {
        <Router initial_path="/">
            <Div>
                <MacroRouteKeyNav />
                <Routes fallback=MacroRouteHomePage>
                    <Route path="/" view=MacroRouteHomePage />
                    <Route path="/counter" view=MacroRouteCounterPage />
                    <Route path="/settings" view=MacroRouteSettingsPage />
                </Routes>
            </Div>
        </Router>
    }
}

/// Descendant component that navigates by updating route context.
#[component]
fn MacroRouteKeyNav() -> impl leptatui::IntoView {
    let navigate = leptatui::use_navigate();

    use_key_event(KeyEventKind::Press, move |key| {
        match key.code {
            KeyCode::Char('h') => navigate("/", NavigateOptions::default()),
            KeyCode::Char('c') => navigate("/counter", NavigateOptions::default()),
            KeyCode::Char('s') => navigate("/settings", NavigateOptions::default()),
            _ => return KeyControl::Pass,
        }

        KeyControl::Handled
    });

    text("Route keys")
}

/// Home page branch for route switching tests.
#[component]
fn MacroRouteHomePage() -> impl leptatui::IntoView {
    MACRO_ROUTE_HOME_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let shared = leptatui::context::expect_context::<MacroSharedCount>().0;

    view! {
        <Div>
            {move || text(format!("Home {}", shared.get()))}
        </Div>
    }
}

/// Counter page branch for route switching tests.
#[component]
fn MacroRouteCounterPage() -> impl leptatui::IntoView {
    MACRO_ROUTE_COUNTER_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let shared = leptatui::context::expect_context::<MacroSharedCount>().0;

    view! {
        <Div>
            {move || text(format!("Counter {}", shared.get()))}
        </Div>
    }
}

/// Settings page branch for route switching tests.
#[component]
fn MacroRouteSettingsPage() -> impl leptatui::IntoView {
    MACRO_ROUTE_SETTINGS_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let shared = leptatui::context::expect_context::<MacroSharedCount>().0;

    view! {
        <Div>
            {move || text(format!("Settings {}", shared.get()))}
        </Div>
    }
}

/// Root component that switches between branches using the same component type.
#[component]
fn MacroRoutePropSwitchRoot() -> impl leptatui::IntoView {
    view! {
        <Router initial_path="/">
            <Div>
                <MacroRoutePropNav />
                <Routes fallback={|| MacroRouteNamedPage::with_props(
                    MacroRouteNamedPageProps::builder().label("Missing").build()
                )}>
                    <Route path="/" view={|| MacroRouteNamedPage::with_props(
                        MacroRouteNamedPageProps::builder().label("Home").build()
                    )} />
                    <Route path="/counter" view={|| MacroRouteNamedPage::with_props(
                        MacroRouteNamedPageProps::builder().label("Counter").build()
                    )} />
                    <Route path="/settings" view={|| MacroRouteNamedPage::with_props(
                        MacroRouteNamedPageProps::builder().label("Settings").build()
                    )} />
                </Routes>
            </Div>
        </Router>
    }
}

/// Descendant key handler for prop-bearing route pages.
#[component]
fn MacroRoutePropNav() -> impl leptatui::IntoView {
    let navigate = leptatui::use_navigate();
    use_key_event(KeyEventKind::Press, move |key| {
        match key.code {
            KeyCode::Char('h') => navigate("/", NavigateOptions::default()),
            KeyCode::Char('c') => navigate("/counter", NavigateOptions::default()),
            KeyCode::Char('s') => navigate("/settings", NavigateOptions::default()),
            _ => return KeyControl::Pass,
        }
        KeyControl::Handled
    });
    text("Route keys")
}

/// Route page whose prop must update when branches share this component type.
#[component]
fn MacroRouteNamedPage(#[prop(into)] label: String) -> impl leptatui::IntoView {
    view! { <Text>{label}</Text> }
}

/// Renders active and inactive route anchors inside their required router context.
///
/// # Returns
///
/// A routed view containing two focusable anchors.
#[component]
fn MacroRouteLinkRoot() -> impl leptatui::IntoView {
    stylesheet! {
        .active => { bg: Color::Red }
    }

    view! {
        <Router initial_path="/docs">
            <Div>
                <A href="/" exact=true>"Home"</A>
                <A href="/docs" exact=true>"Docs"</A>
            </Div>
        </Router>
    }
}
