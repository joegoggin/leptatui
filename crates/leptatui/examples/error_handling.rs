//! Fallible component and panic-cleanup showcase.
//!
//! This binary demonstrates `?` propagation, custom `view_error!` messages,
//! retained anyhow source context, router-aware error recovery, and terminal
//! restoration before panic diagnostics.

use leptatui::prelude::*;

/// Root component for the error-handling showcase.
///
/// # Returns
///
/// A routed application view containing every error demonstration.
#[component]
fn ErrorHandlingExample() -> impl IntoView {
    stylesheet! {
        .shell => {
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }
        .title => { fg: Color::LightCyan, modifier: Modifier::BOLD }
        .nav => {
            display: Display::Flex,
            flex_direction: FlexDirection::Row
        }
        .page => {
            borders: Borders::ALL,
            padding: TuiSpacing::uniform(1)
        }
        .warning => { fg: Color::LightYellow }
        .danger => { fg: Color::LightRed, modifier: Modifier::BOLD }

        @media (max-width: 60) {
            .shell => { padding: TuiSpacing::ZERO }
            .nav => { flex_direction: FlexDirection::Column }
            .page => { padding: TuiSpacing::ZERO }
        }
    }

    view! {
        <Router initial_path="/">
            <Block class="shell">
                <Div>
                    <Text class="title">"Leptatui error handling"</Text>
                    <NavigationKeys />
                    <Div class="nav">
                        <A href="/" exact=true>
                            "Home"
                        </A>
                        <A href="/propagated">"Propagated error"</A>
                        <A href="/custom">"Custom error"</A>
                        <A href="/source">"Context error"</A>
                        <A href="/panic">"Panic cleanup"</A>
                    </Div>
                    <Routes fallback=HomePage>
                        <Route path="/" view=HomePage />
                        <Route path="/propagated" view=PropagatedErrorPage />
                        <Route path="/custom" view=CustomErrorPage />
                        <Route path="/source" view=ContextErrorPage />
                        <Route path="/panic" view=PanicPage />
                    </Routes>
                    <Text>"h home | e propagated | c custom | s source | p panic | q quit"</Text>
                </Div>
            </Block>
        </Router>
    }
}

/// Installs route and quit shortcuts inside router context.
///
/// # Returns
///
/// An empty view that owns the registered keyboard handler.
#[component]
fn NavigationKeys() -> impl IntoView {
    let navigate = use_navigate();
    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('h') => {
            navigate("/", NavigateOptions::default());
            KeyControl::Handled
        }
        KeyCode::Char('e') => {
            navigate("/propagated", NavigateOptions::default());
            KeyControl::Handled
        }
        KeyCode::Char('c') => {
            navigate("/custom", NavigateOptions::default());
            KeyControl::Handled
        }
        KeyCode::Char('s') => {
            navigate("/source", NavigateOptions::default());
            KeyControl::Handled
        }
        KeyCode::Char('p') => {
            navigate("/panic", NavigateOptions::default());
            KeyControl::Handled
        }
        KeyCode::Char('q') => KeyControl::Exit,
        _ => KeyControl::Pass,
    });
    div(())
}

/// Landing page explaining the available demonstrations.
///
/// # Returns
///
/// A page describing recoverable errors and panic cleanup.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <Div class="page">
            <H1>"Choose an error path"</H1>
            <Paragraph>
                "The recoverable pages open Leptatui's default error screen. Use Back to return here or Quit to exit cleanly."
            </Paragraph>
            <Paragraph>
                "The panic page restores the normal terminal before Rust prints its panic diagnostics."
            </Paragraph>
        </Div>
    }
}

/// Page that propagates an ordinary I/O error with `?`.
///
/// # Returns
///
/// A view when the demonstration data loads successfully.
///
/// # Errors
///
/// Returns [`ViewError`] when the demonstration data fails to load.
#[component]
fn PropagatedErrorPage() -> ViewResult<impl IntoView> {
    load_demo_data()?;
    view! { <Text>"This view is never reached."</Text> }
}

/// Page that replaces an operation failure with a custom message.
///
/// # Returns
///
/// A view when the demonstration data loads successfully.
///
/// # Errors
///
/// Returns [`ViewError`] with a custom message when loading fails.
#[component]
fn CustomErrorPage() -> ViewResult<impl IntoView> {
    if load_demo_data().is_err() {
        view_error!("The custom demo could not load its required data.");
    }
    view! { <Text>"This view is never reached."</Text> }
}

/// Page that preserves an operation failure beneath custom context.
///
/// # Returns
///
/// A view when the demonstration data loads successfully.
///
/// # Errors
///
/// Returns [`ViewError`] containing custom context and the loading failure.
#[component]
fn ContextErrorPage() -> ViewResult<impl IntoView> {
    if let Err(error) = load_demo_data() {
        view_error!(error => "The context demo failed while loading its data");
    }
    view! { <Text>"This view is never reached."</Text> }
}

/// Page containing an explicit panic trigger.
///
/// # Returns
///
/// A page with a button that triggers the panic-cleanup path.
#[component]
fn PanicPage() -> impl IntoView {
    view! {
        <Div class="page">
            <H1 class="danger">"Panic cleanup"</H1>
            <Paragraph class="warning">
                "Activating this button intentionally panics. Leptatui will restore the terminal before Rust prints the diagnostic."
            </Paragraph>
            <Button on_press=|| {
                panic!("intentional error_handling example panic")
            }>"Trigger panic"</Button>
        </Div>
    }
}

/// Reads the intentionally absent file used by the recoverable examples.
///
/// # Returns
///
/// A UTF-8 [`String`] containing the demonstration data.
///
/// # Errors
///
/// Returns [`std::io::Error`] because `examples/demo-data.json` is
/// intentionally absent, or if the file cannot otherwise be read as UTF-8.
fn load_demo_data() -> std::io::Result<String> {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/demo-data.json"
    ))
}

/// Runs the error-handling showcase.
///
/// # Returns
///
/// An empty [`leptatui::Result`] when the application exits successfully.
///
/// # Errors
///
/// Returns [`leptatui::Error::Io`] if terminal setup, rendering, input, or
/// cleanup fails. Returns [`leptatui::Error::EventTask`] if event polling
/// fails.
#[tokio::main]
async fn main() -> leptatui::Result<()> {
    App::new(view! { <ErrorHandlingExample /> }).run().await
}
