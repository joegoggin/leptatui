//! Multi-page routing demo.
//!
//! This binary demonstrates route-driven page switching, shared context state,
//! component pages, stylesheets, and keyboard interaction through the app
//! runner.

use leptatui::prelude::*;

/// Pages available in the demo router.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoPage {
    Home,
    Counter,
    Settings,
}

/// Theme preference shared by the demo pages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
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
                .color("muted", Color::DarkGray)
                .color("surface", Color::White)
                .color("panel", Color::White)
                .color("accent", Color::Blue)
                .color("focus_text", Color::Black)
                .color("focus_surface", Color::Yellow),
            Self::Dark => ThemeVariables::new()
                .color("text", Color::White)
                .color("muted", Color::Gray)
                .color("surface", Color::Black)
                .color("panel", Color::Black)
                .color("accent", Color::LightCyan)
                .color("focus_text", Color::Black)
                .color("focus_surface", Color::LightCyan),
        }
    }
}

/// Navigates to a page and keeps the app running.
fn navigate_to(navigate: WriteSignal<DemoPage>, page: DemoPage) -> AppControl {
    navigate.update(|route| *route = page);
    AppControl::Continue
}

/// Root component for the multi-page demo.
#[component]
fn MultiPageDemo() -> View {
    let counter = RwSignal::new(0);
    let theme_mode = RwSignal::new(ThemeMode::Light);
    let theme = RwSignal::new(ThemeMode::Light.variables());
    let route_state = provide_route(DemoPage::Home);
    let route = route_state.route();

    provide_context(counter);
    provide_context(theme_mode);
    provide_context(theme);
    provide_context(theme.read_only());

    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        $text: theme_color("text");
        $muted: theme_color("muted");
        $surface: theme_color("surface");
        $panel: theme_color("panel");
        $accent: theme_color("accent");
        $focus_text: theme_color("focus_text");
        $focus_surface: theme_color("focus_surface");

        .app-shell => {
            fg: $text,
            bg: $surface,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }

        .app-title => { fg: $accent, modifier: Modifier::BOLD }
        .nav => { direction: LayoutDirection::Row }
        .page => { fg: $text, bg: $panel, padding: TuiSpacing::uniform(1) }
        .page-title => { fg: $accent, modifier: Modifier::BOLD }
        .body => { fg: $text }
        .muted => { fg: $muted }
        .stat => { fg: $accent, modifier: Modifier::BOLD }
        .danger => { fg: Color::LightRed }

        Button => {
            fg: $text,
            bg: $surface,
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1),

            &:focus => {
                fg: $focus_text,
                bg: $focus_surface,
                modifier: Modifier::BOLD,
                border_type: BorderType::Thick
            }
        }

        @media (max-width: 60) {
            .app-shell => { padding: TuiSpacing::ZERO }
            .nav => { direction: LayoutDirection::Column }
            .actions => { direction: LayoutDirection::Column }
            .page => { padding: TuiSpacing::ZERO }
            Button => { padding: TuiSpacing::ZERO }
        }
    }

    view! {
        <Block class="app-shell">
            <Column>
                <Text class="app-title">"Leptatui multi-page demo"</Text>
                <Nav />
                {move || match route.get_untracked() {
                    DemoPage::Home => view! { <HomePage /> },
                    DemoPage::Counter => view! { <CounterPage /> },
                    DemoPage::Settings => view! { <SettingsPage /> },
                }}
                <Text class="muted">
                    "h Home | c Counter | s Settings | +/- count | t theme | q quit"
                </Text>
            </Column>
        </Block>
    }
}

/// Top navigation shared across pages.
#[component]
fn Nav() -> View {
    let navigate = use_navigate::<DemoPage>();

    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('h') => {
            navigate.update(|route| *route = DemoPage::Home);
            KeyControl::Handled
        }
        KeyCode::Char('c') => {
            navigate.update(|route| *route = DemoPage::Counter);
            KeyControl::Handled
        }
        KeyCode::Char('s') => {
            navigate.update(|route| *route = DemoPage::Settings);
            KeyControl::Handled
        }
        _ => KeyControl::Pass,
    });

    let home = use_navigate::<DemoPage>();
    let counter = use_navigate::<DemoPage>();
    let settings = use_navigate::<DemoPage>();

    view! {
        <Row class="nav">
            <Button on_press=move || navigate_to(home, DemoPage::Home)>"Home"</Button>
            <Button on_press=move || navigate_to(counter, DemoPage::Counter)>"Counter"</Button>
            <Button on_press=move || navigate_to(settings, DemoPage::Settings)>"Settings"</Button>
        </Row>
    }
}

/// Landing page that summarizes shared demo state.
#[component]
fn HomePage() -> View {
    let counter = expect_context::<RwSignal<i32>>();
    let theme_mode = expect_context::<RwSignal<ThemeMode>>();

    view! {
        <Block class="page">
            <Column>
                <Text class="page-title">"Home"</Text>
                <Text class="body">
                    "This page reads the same shared app state as the other routes."
                </Text>
                {move || {
                    view! {
                        <Text class="stat">
                            {format!(
                                "Count: {} | Theme: {:?}",
                                counter.get_untracked(),
                                theme_mode.get_untracked(),
                            )}
                        </Text>
                    }
                }}
                <Text class="muted">
                    "Use c for Counter, s for Settings, or Tab to focus the nav buttons."
                </Text>
            </Column>
        </Block>
    }
}

/// Counter page that remains interactive after route navigation.
#[component]
fn CounterPage() -> View {
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
        <Block class="page">
            <Column>
                <Text class="page-title">"Counter"</Text>
                {move || {
                    view! {
                        <Text class="stat">{format!("Count: {}", counter.get_untracked())}</Text>
                    }
                }}
                <Row class="actions">
                    <Button on_press=move || {
                        counter.update(|count| *count += 1);
                        AppControl::Continue
                    }>"+ Increment"</Button>
                    <Button on_press=move || {
                        counter.update(|count| *count -= 1);
                        AppControl::Continue
                    }>"- Decrement"</Button>
                    <Button on_press=move || {
                        counter.set(0);
                        AppControl::Continue
                    }>"Reset"</Button>
                </Row>
                <Text class="muted">"+/- adjusts the shared count. r resets it."</Text>
            </Column>
        </Block>
    }
}

/// Settings page that updates shared theme preference state.
#[component]
fn SettingsPage() -> View {
    let mode = expect_context::<RwSignal<ThemeMode>>();
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
        <Block class="page">
            <Column>
                <Text class="page-title">"Settings"</Text>
                {move || {
                    view! {
                        <Text class="stat">
                            {format!("Theme preference: {:?}", mode.get_untracked())}
                        </Text>
                    }
                }}
                <Text class="body">
                    "Theme variables are shared through context and resolved by the stylesheet."
                </Text>
                <Row class="actions">
                    <Button on_press=move || {
                        mode.update(|mode| {
                            *mode = mode.toggle();
                            theme.set(mode.variables());
                        });
                        AppControl::Continue
                    }>"Toggle theme"</Button>
                    <Button class="danger" on_press=|| AppControl::Exit>
                        "Quit"
                    </Button>
                </Row>
                <Text class="muted">"Press t to toggle theme without leaving Settings."</Text>
            </Column>
        </Block>
    }
}

/// Runs the multi-page demo application.
#[tokio::main]
async fn main() -> Result<()> {
    App::new(MultiPageDemo::new()).run().await
}
