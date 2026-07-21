/// Root component that renders page branches from route state.
#[component]
fn MacroRouteSwitchRoot() -> impl leptatui::IntoView {
    MACRO_ROUTE_ROOT_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);

    let (shared_count, set_shared_count) = signal(0);
    provide_context(MacroSharedCount(shared_count));
    let route_state = leptatui::provide_route(MacroRoutePage::Home);
    let route = route_state.route();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('i') {
            set_shared_count.update(|count| *count += 1);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    view! {
        <Column>
            <MacroRouteKeyNav />
            {move || match route.get_untracked() {
                MacroRoutePage::Home => view! { <MacroRouteHomePage /> },
                MacroRoutePage::Counter => view! { <MacroRouteCounterPage /> },
                MacroRoutePage::Settings => view! { <MacroRouteSettingsPage /> },
            }}
        </Column>
    }
}

/// Descendant component that navigates by updating route context.
#[component]
fn MacroRouteKeyNav() -> impl leptatui::IntoView {
    let navigate = leptatui::use_navigate::<MacroRoutePage>();

    use_key_event(KeyEventKind::Press, move |key| {
        match key.code {
            KeyCode::Char('h') => navigate.update(|route| *route = MacroRoutePage::Home),
            KeyCode::Char('c') => navigate.update(|route| *route = MacroRoutePage::Counter),
            KeyCode::Char('s') => navigate.update(|route| *route = MacroRoutePage::Settings),
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
        <Column>
            {move || text(format!("Home {}", shared.get_untracked()))}
        </Column>
    }
}

/// Counter page branch for route switching tests.
#[component]
fn MacroRouteCounterPage() -> impl leptatui::IntoView {
    MACRO_ROUTE_COUNTER_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let shared = leptatui::context::expect_context::<MacroSharedCount>().0;

    view! {
        <Column>
            {move || text(format!("Counter {}", shared.get_untracked()))}
        </Column>
    }
}

/// Settings page branch for route switching tests.
#[component]
fn MacroRouteSettingsPage() -> impl leptatui::IntoView {
    MACRO_ROUTE_SETTINGS_SETUP_RUNS.fetch_add(1, Ordering::SeqCst);
    let shared = leptatui::context::expect_context::<MacroSharedCount>().0;

    view! {
        <Column>
            {move || text(format!("Settings {}", shared.get_untracked()))}
        </Column>
    }
}

/// Root component that switches between branches using the same component type.
#[component]
fn MacroRoutePropSwitchRoot() -> impl leptatui::IntoView {
    let route_state = leptatui::provide_route(MacroRoutePage::Home);
    let route = route_state.route();
    let navigate = route_state.navigate();

    use_key_event(KeyEventKind::Press, move |key| {
        match key.code {
            KeyCode::Char('h') => navigate.update(|route| *route = MacroRoutePage::Home),
            KeyCode::Char('c') => navigate.update(|route| *route = MacroRoutePage::Counter),
            KeyCode::Char('s') => navigate.update(|route| *route = MacroRoutePage::Settings),
            _ => return KeyControl::Pass,
        }

        KeyControl::Handled
    });

    view! {
        <Column>
            {move || match route.get_untracked() {
                MacroRoutePage::Home => view! { <MacroRouteNamedPage label="Home" /> },
                MacroRoutePage::Counter => view! { <MacroRouteNamedPage label="Counter" /> },
                MacroRoutePage::Settings => view! { <MacroRouteNamedPage label="Settings" /> },
            }}
        </Column>
    }
}

/// Route page whose prop must update when branches share this component type.
#[component]
fn MacroRouteNamedPage(#[prop(into)] label: String) -> impl leptatui::IntoView {
    view! { <Text>{label}</Text> }
}
