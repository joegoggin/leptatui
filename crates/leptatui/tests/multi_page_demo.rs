//! Multi-page demo workflow tests.
//!
//! These tests exercise the public routing, shared state, and component style
//! used by the `multi_page_demo` example without running an interactive
//! terminal session.

use std::process::Command;

use crossterm::event::KeyCode;
use leptatui::prelude::*;

mod support;

use support::{key, render_component, rendered_text};

/// Test routes matching the demo page shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoTestPage {
    Home,
    Counter,
    Settings,
}

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
fn DemoWorkflowRoot() -> View {
    let counter = RwSignal::new(0);
    let theme_mode = RwSignal::new(DemoTestTheme::Light);
    let theme = RwSignal::new(DemoTestTheme::Light.variables());
    let route_state = provide_route(DemoTestPage::Home);
    let route = route_state.route();

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
        <Column>
            <DemoWorkflowNav />
            {move || match route.get_untracked() {
                DemoTestPage::Home => view! { <DemoWorkflowHome /> },
                DemoTestPage::Counter => view! { <DemoWorkflowCounter /> },
                DemoTestPage::Settings => view! { <DemoWorkflowSettings /> },
            }}
        </Column>
    }
}

/// Navigation component using route context.
#[component]
fn DemoWorkflowNav() -> View {
    let navigate = use_navigate::<DemoTestPage>();

    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('h') => {
            navigate.update(|route| *route = DemoTestPage::Home);
            KeyControl::Handled
        }
        KeyCode::Char('c') => {
            navigate.update(|route| *route = DemoTestPage::Counter);
            KeyControl::Handled
        }
        KeyCode::Char('s') => {
            navigate.update(|route| *route = DemoTestPage::Settings);
            KeyControl::Handled
        }
        _ => KeyControl::Pass,
    });

    view! {
        <Row>
            <Button>"Home"</Button>
            <Button>"Counter"</Button>
            <Button>"Settings"</Button>
        </Row>
    }
}

/// Home page that reads shared state.
#[component]
fn DemoWorkflowHome() -> View {
    let counter = expect_context::<RwSignal<i32>>();
    let theme_mode = expect_context::<RwSignal<DemoTestTheme>>();

    view! {
        <Column class="page">
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
        </Column>
    }
}

/// Counter page that updates shared counter state.
#[component]
fn DemoWorkflowCounter() -> View {
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
        <Column class="page">
            <Text class="title">"Counter"</Text>
            {move || view! { <Text>{format!("Count: {}", counter.get_untracked())}</Text> }}
        </Column>
    }
}

/// Settings page that updates shared theme state.
#[component]
fn DemoWorkflowSettings() -> View {
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
        <Column class="page">
            <Text class="title">"Settings"</Text>
            {move || view! { <Text>{format!("Theme: {:?}", mode.get_untracked())}</Text> }}
        </Column>
    }
}

/// Verifies the demo workflow routes between pages while preserving shared state.
#[test]
fn multi_page_demo_routes_counter_and_theme_state() -> Result<()> {
    let mut component = DemoWorkflowRoot::new();

    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Home"), "rendered text: {text:?}");
    assert!(
        text.contains("Count: 0 | Theme: Light"),
        "rendered text: {text:?}"
    );

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('c')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Counter"), "rendered text: {text:?}");
    assert!(text.contains("Count: 0"), "rendered text: {text:?}");

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('+')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Counter"), "rendered text: {text:?}");
    assert!(text.contains("Count: 1"), "rendered text: {text:?}");

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('s')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Settings"), "rendered text: {text:?}");
    assert!(text.contains("Theme: Light"), "rendered text: {text:?}");

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('t')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Settings"), "rendered text: {text:?}");
    assert!(text.contains("Theme: Dark"), "rendered text: {text:?}");

    assert_eq!(
        Component::handle_event(&mut component, key(KeyCode::Char('h')))?,
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
        Component::handle_event(&mut component, key(KeyCode::Char('c')))?,
        AppControl::Continue
    );
    let terminal = render_component(&mut component, 48, 6)?;
    let text = rendered_text(&terminal);
    assert!(text.contains("Counter"), "rendered text: {text:?}");
    assert!(text.contains("Count: 1"), "rendered text: {text:?}");

    Ok(())
}

/// Verifies the runnable multi-page demo example compiles.
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
