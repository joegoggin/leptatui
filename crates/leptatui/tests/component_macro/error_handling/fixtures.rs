/// Component that replaces a failed operation with a custom message.
///
/// # Returns
///
/// A fallible view result containing the replacement diagnostic.
///
/// # Errors
///
/// Returns [`leptatui::ViewError`] with the custom display message.
#[component]
fn MacroReplacedError() -> leptatui::ViewResult<impl leptatui::IntoView> {
    stylesheet! {
        Block => {
            borders: Borders::NONE !important,
            bg: Color::Blue !important
        }
        Div => { display: Display::None !important }
        H1 => { fg: Color::Green !important }
        Paragraph => { fg: Color::Yellow !important }
        Button => { display: Display::None !important }
    }

    let operation: std::io::Result<()> = Err(std::io::Error::other("hidden source"));
    if operation.is_err() {
        leptatui::view_error!("custom display message");
    }
    text("unreachable")
}

/// Component that produces a long diagnostic for constrained-screen rendering.
///
/// # Returns
///
/// A fallible view result containing the long diagnostic.
///
/// # Errors
///
/// Returns [`leptatui::ViewError`] with a message wider than the test viewport.
#[component]
fn MacroLongError() -> leptatui::ViewResult<impl leptatui::IntoView> {
    let operation: std::io::Result<()> = Err(std::io::Error::other("long diagnostic"));
    if operation.is_err() {
        leptatui::view_error!(
            "This intentionally long error message wraps across several terminal rows without moving the recovery controls off screen."
        );
    }

    text("unreachable")
}

/// Component that retains a failed operation beneath custom context.
///
/// # Returns
///
/// A fallible view result containing the contextual diagnostic.
///
/// # Errors
///
/// Returns [`leptatui::ViewError`] containing context and source messages.
#[component]
fn MacroContextError() -> leptatui::ViewResult<impl leptatui::IntoView> {
    let operation: std::io::Result<()> = Err(std::io::Error::other("source detail"));
    if let Err(error) = operation {
        leptatui::view_error!(error => "custom context");
    }
    text("unreachable")
}

/// Routed root used to verify error-screen history recovery.
///
/// # Returns
///
/// A routed fixture containing healthy and failing pages.
#[component]
fn MacroErrorRouteRoot() -> impl leptatui::IntoView {
    stylesheet! {
        Block => {
            borders: Borders::NONE !important,
            bg: Color::Blue !important
        }
        H1 => { display: Display::None !important }
        Paragraph => { display: Display::None !important }
        Button => { display: Display::None !important }
    }

    view! {
        <Router initial_path="/">
            <Div>
                <MacroErrorRouteNavigation />
                <Routes fallback=MacroErrorRouteHome>
                    <Route path="/" view=MacroErrorRouteHome />
                    <Route path="/failure" view=MacroErrorRouteFailure />
                </Routes>
            </Div>
        </Router>
    }
}

/// Routed shell used to verify an error replaces every surrounding symbol.
///
/// # Returns
///
/// A bordered application shell containing navigation, routes, and a footer.
#[component]
fn MacroShellErrorRoot() -> impl leptatui::IntoView {
    stylesheet! {
        .shell => { fg: Color::LightCyan }
    }

    view! {
        <Router initial_path="/">
            <Block class="shell">
                <Div>
                    <Text>"Leptatui error handling"</Text>
                    <MacroErrorRouteNavigation />
                    <Div>
                        <A href="/" exact=true>"Home"</A>
                        <A href="/failure">"Propagated error"</A>
                    </Div>
                    <Routes fallback=MacroErrorRouteHome>
                        <Route path="/" view=MacroErrorRouteHome />
                        <Route path="/failure" view=MacroErrorRouteFailure />
                    </Routes>
                    <Text>"h home | e propagated | q quit"</Text>
                </Div>
            </Block>
        </Router>
    }
}

/// Keyboard navigation used by the routed error fixture.
///
/// # Returns
///
/// A label view that owns the route keyboard handler.
#[component]
fn MacroErrorRouteNavigation() -> impl leptatui::IntoView {
    let navigate = leptatui::use_navigate();
    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('e') {
            navigate("/failure", NavigateOptions::default());
            return KeyControl::Handled;
        }
        KeyControl::Pass
    });
    text("Error route navigation")
}

/// Successful page preceding the routed error.
///
/// # Returns
///
/// A healthy-page label view.
#[component]
fn MacroErrorRouteHome() -> impl leptatui::IntoView {
    text("Healthy page")
}

/// Fallible route that always opens the default error screen.
///
/// # Returns
///
/// A view when the synthetic route operation succeeds.
///
/// # Errors
///
/// Returns [`leptatui::ViewError`] for every invocation.
#[component]
fn MacroErrorRouteFailure() -> leptatui::ViewResult<impl leptatui::IntoView> {
    macro_error_route_failure()?;
    text("unreachable")
}

/// Returns the synthetic error used by the routed error fixture.
///
/// # Returns
///
/// An empty result when the synthetic operation succeeds.
///
/// # Errors
///
/// Returns [`std::io::Error`] for every invocation.
fn macro_error_route_failure() -> std::io::Result<()> {
    Err(std::io::Error::other("routed failure"))
}
