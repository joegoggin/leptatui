//! Multi-page demo workflow tests.
//!
//! These tests exercise the public routing, shared state, and component style
//! used by the `multi_page_demo` example without running an interactive
//! terminal session.
//!
//! # Modules
//!
//! - [`support`] — Shared component rendering and key-event helpers.

use std::process::Command;

use crossterm::event::KeyCode;
use leptatui::prelude::*;

mod support;

use support::{key, render_component, rendered_text};

/// Test theme preference shared across pages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoTestTheme {
    Light,
    Dark,
}

impl DemoTestTheme {
    /// Returns the opposite theme preference.
    fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    /// Returns theme variables for this preference.
    fn variables(self) -> ThemeVariables {
        match self {
            Self::Light => ThemeVariables::new()
                .color("text", Color::Black)
                .color("surface", Color::White)
                .color("accent", Color::Blue),
            Self::Dark => ThemeVariables::new()
                .color("text", Color::White)
                .color("surface", Color::Black)
                .color("accent", Color::LightCyan),
        }
    }
}

/// Root component that mirrors the multi-page demo's route and shared state.
#[component]
fn DemoWorkflowRoot() -> impl IntoView {
    let counter = RwSignal::new(0);
    let theme_mode = RwSignal::new(DemoTestTheme::Light);
    let theme = RwSignal::new(DemoTestTheme::Light.variables());
    provide_context(counter);
    provide_context(theme_mode);
    provide_context(theme);
    provide_context(theme.read_only());

    stylesheet! {
        $text: theme_color("text");
        $surface: theme_color("surface");
        $accent: theme_color("accent");

        .page => { fg: $text, bg: $surface }
        .title => { fg: $accent, modifier: Modifier::BOLD }
        Button => { borders: Borders::ALL }
    }

    view! {
        <Router initial_path="/">
            <Div>
                <DemoWorkflowNav />
                <Routes fallback=DemoWorkflowNotFound>
                    <Route path="/" view=DemoWorkflowHome />
                    <Route path="/counter" view=DemoWorkflowCounter />
                    <ParentRoute path="/settings" view=DemoWorkflowSettingsLayout>
                        <Route path="theme" view=DemoWorkflowSettings />
                    </ParentRoute>
                </Routes>
            </Div>
        </Router>
    }
}

/// Navigation component using route context.
#[component]
fn DemoWorkflowNav() -> impl IntoView {
    let navigate = use_navigate();

    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('h') => {
            navigate("/", NavigateOptions::default());
            KeyControl::Handled
        }
        KeyCode::Char('c') => {
            navigate("/counter", NavigateOptions::default());
            KeyControl::Handled
        }
        KeyCode::Char('s') => {
            navigate("/settings/theme", NavigateOptions::default());
            KeyControl::Handled
        }
        _ => KeyControl::Pass,
    });

    view! {
        <Div style={TuiStyle::new().display(Display::Flex)}>
            <Button>"Home"</Button>
            <Button>"Counter"</Button>
            <Button>"Settings"</Button>
        </Div>
    }
}

/// Renders the test router fallback.
///
/// # Returns
///
/// A not-found text component.
#[component]
fn DemoWorkflowNotFound() -> impl IntoView {
    text("Not found")
}

/// Renders a retained settings layout around its child route.
///
/// # Returns
///
/// A settings heading followed by the current [`Outlet`].
#[component]
fn DemoWorkflowSettingsLayout() -> impl IntoView {
    view! {
        <Div>
            <Text>"Settings layout"</Text>
            <Outlet />
        </Div>
    }
}

/// Home page that reads shared state.
#[component]
fn DemoWorkflowHome() -> impl IntoView {
    let counter = expect_context::<RwSignal<i32>>();
    let theme_mode = expect_context::<RwSignal<DemoTestTheme>>();

    view! {
        <Div class="page">
            <Text class="title">"Home"</Text>
            {move || {
                view! {
                    <Text>
                        {format!(
                            "Count: {} | Theme: {:?}",
                            counter.get_untracked(),
                            theme_mode.get_untracked(),
                        )}
                    </Text>
                }
            }}
        </Div>
    }
}

/// Counter page that updates shared counter state.
#[component]
fn DemoWorkflowCounter() -> impl IntoView {
    let counter = expect_context::<RwSignal<i32>>();

    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('+') | KeyCode::Char('=') => {
            counter.update(|count| *count += 1);
            KeyControl::Handled
        }
        KeyCode::Char('-') => {
            counter.update(|count| *count -= 1);
            KeyControl::Handled
        }
        KeyCode::Char('r') => {
            counter.set(0);
            KeyControl::Handled
        }
        _ => KeyControl::Pass,
    });

    view! {
        <Div class="page">
            <Text class="title">"Counter"</Text>
            {move || view! { <Text>{format!("Count: {}", counter.get_untracked())}</Text> }}
        </Div>
    }
}

/// Settings page that updates shared theme state.
#[component]
fn DemoWorkflowSettings() -> impl IntoView {
    let mode = expect_context::<RwSignal<DemoTestTheme>>();
    let theme = expect_context::<RwSignal<ThemeVariables>>();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('t') {
            mode.update(|mode| {
                *mode = mode.toggle();
                theme.set(mode.variables());
            });
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    view! {
        <Div class="page">
            <Text class="title">"Settings"</Text>
            {move || view! { <Text>{format!("Theme: {:?}", mode.get_untracked())}</Text> }}
        </Div>
    }
}

/// Verifies the demo workflow routes between pages while preserving shared state.
///
/// # Example Under Test
///
/// ```text
/// Home -> Counter (+) -> Settings (toggle) -> Home -> Counter
/// ```
///
/// # Assertions
///
/// - Each navigation key selects the expected page.
/// - Counter changes remain visible after navigating away and back.
/// - Theme changes remain visible after navigating away and back.
/// - Every handled workflow key returns `AppControl::Continue`.
#[test]
fn multi_page_demo_routes_counter_and_theme_state() -> leptatui::app::Result<()> {
    let mut component = DemoWorkflowRoot::new();

    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Home"), "rendered text: {text:?}");
    assert!(
        text.contains("Count: 0 | Theme: Light"),
        "rendered text: {text:?}"
    );

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('c')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Counter"), "rendered text: {text:?}");
    assert!(text.contains("Count: 0"), "rendered text: {text:?}");

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('+')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Counter"), "rendered text: {text:?}");
    assert!(text.contains("Count: 1"), "rendered text: {text:?}");

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('s')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Settings"), "rendered text: {text:?}");
    assert!(text.contains("Theme: Light"), "rendered text: {text:?}");

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('t')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Settings"), "rendered text: {text:?}");
    assert!(text.contains("Theme: Dark"), "rendered text: {text:?}");

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('h')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Home"), "rendered text: {text:?}");
    assert!(
        text.contains("Count: 1 | Theme: Dark"),
        "rendered text: {text:?}"
    );

    assert_eq!(
        View::handle_event(&mut component, key(KeyCode::Char('c')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Counter"), "rendered text: {text:?}");
    assert!(text.contains("Count: 1"), "rendered text: {text:?}");

    Ok(())
}

/// Verifies the runnable multi-page demo example compiles.
///
/// # Example Under Test
///
/// ```text
/// cargo check --quiet --example multi_page_demo
/// ```
///
/// # Assertions
///
/// - Cargo launches successfully for the example target.
/// - The example target exits with a successful status.
#[test]
fn multi_page_demo_example_compiles() {
    let output = Command::new("cargo")
        .args(["check", "--quiet", "--example", "multi_page_demo"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo check should run for multi_page_demo");

    assert!(
        output.status.success(),
        "cargo check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
