//! Multi-page routing demo.
//!
//! This binary demonstrates route-driven page switching, shared context state,
//! component pages, stylesheets, and keyboard interaction through the app
//! runner.

use leptatui::prelude::*;

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

/// Root component for the multi-page demo.
#[component]
fn MultiPageDemo() -> impl IntoView {
    let counter = RwSignal::new(0);
    let theme_mode = RwSignal::new(ThemeMode::Light);
    let theme = RwSignal::new(ThemeMode::Light.variables());
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
        .nav => {
            display: Display::Flex,
            flex_direction: FlexDirection::Row
        }
        .actions => { display: Display::Flex }
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
            .nav => { flex_direction: FlexDirection::Column }
            .actions => { flex_direction: FlexDirection::Column }
            .page => { padding: TuiSpacing::ZERO }
            Button => { padding: TuiSpacing::ZERO }
        }
    }

    view! {
        <Router initial_path="/">
            <Block class="app-shell">
                <Div>
                    <Text class="app-title">"Leptatui multi-page demo"</Text>
                    <Nav />
                    <Routes fallback=NotFoundPage>
                        <Route path="/" view=HomePage />
                        <Route path="/counter" view=CounterPage />
                        <Route path="/settings" view=SettingsPage />
                    </Routes>
                    <Text class="muted">
                        "h Home | c Counter | s Settings | +/- count | t theme | q quit"
                    </Text>
                </Div>
            </Block>
        </Router>
    }
}

/// Top navigation shared across pages.
#[component]
fn Nav() -> impl IntoView {
    let shortcut_navigate = use_navigate();

    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('h') => {
            shortcut_navigate("/", NavigateOptions::default());
            KeyControl::Handled
        }
        KeyCode::Char('c') => {
            shortcut_navigate("/counter", NavigateOptions::default());
            KeyControl::Handled
        }
        KeyCode::Char('s') => {
            shortcut_navigate("/settings", NavigateOptions::default());
            KeyControl::Handled
        }
        _ => KeyControl::Pass,
    });

    view! {
        <Div class="nav">
            <A href="/" exact=true>"Home"</A>
            <A href="/counter">"Counter"</A>
            <A href="/settings">"Settings"</A>
        </Div>
    }
}

/// Renders an unmatched-location fallback.
///
/// # Returns
///
/// A not-found page component.
#[component]
fn NotFoundPage() -> impl IntoView {
    let location = use_location();
    view! {
        <Block class="page">
            <Text class="danger">
                {move || format!("No route matches {}", location.pathname().get())}
            </Text>
        </Block>
    }
}

/// Landing page that summarizes shared demo state.
#[component]
fn HomePage() -> impl IntoView {
    let counter = expect_context::<RwSignal<i32>>();
    let theme_mode = expect_context::<RwSignal<ThemeMode>>();

    view! {
        <Block class="page">
            <Div>
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
            </Div>
        </Block>
    }
}

/// Counter page that remains interactive after route navigation.
#[component]
fn CounterPage() -> impl IntoView {
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
            <Div>
                <Text class="page-title">"Counter"</Text>
                {move || {
                    view! {
                        <Text class="stat">{format!("Count: {}", counter.get_untracked())}</Text>
                    }
                }}
                <Div class="actions">
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
                </Div>
                <Text class="muted">"+/- adjusts the shared count. r resets it."</Text>
            </Div>
        </Block>
    }
}

/// Settings page that updates shared theme preference state.
#[component]
fn SettingsPage() -> impl IntoView {
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
            <Div>
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
                <Div class="actions">
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
                </Div>
                <Text class="muted">"Press t to toggle theme without leaving Settings."</Text>
            </Div>
        </Block>
    }
}

/// Runs the multi-page demo application.
#[tokio::main]
async fn main() -> Result<()> {
    let view = view! { <MultiPageDemo /> };
    App::new(view).run().await
}
