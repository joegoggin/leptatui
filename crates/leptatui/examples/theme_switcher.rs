//! Context-backed light/dark theme switching example.

use leptatui::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Active theme mode selected by the demo.
enum ThemeMode {
    /// Light color palette.
    Light,
    /// Dark color palette.
    Dark,
}

impl ThemeMode {
    /// Returns the opposite theme mode.
    ///
    /// # Returns
    ///
    /// A [`ThemeMode`] containing the next selected mode.
    fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    /// Builds the theme variables for this mode.
    ///
    /// # Returns
    ///
    /// A [`ThemeVariables`] value containing colors for stylesheet resolution.
    fn variables(self) -> ThemeVariables {
        match self {
            Self::Light => ThemeVariables::new()
                .color("text", Color::Black)
                .color("muted", Color::DarkGray)
                .color("surface", Color::White)
                .color("accent", Color::Blue)
                .color("focus_text", Color::Black)
                .color("focus_surface", Color::Yellow),
            Self::Dark => ThemeVariables::new()
                .color("text", Color::White)
                .color("muted", Color::Gray)
                .color("surface", Color::Black)
                .color("accent", Color::LightCyan)
                .color("focus_text", Color::Black)
                .color("focus_surface", Color::LightCyan),
        }
    }
}

/// Renders the currently active theme mode.
///
/// # Returns
///
/// A [`View`] containing the active theme status text.
#[component]
fn ThemeStatus() -> impl IntoView {
    let mode = expect_context::<ReadSignal<ThemeMode>>();

    dynamic(move || {
        view! { <Text class="theme-status">{format!("Active theme: {:?}", mode.get_untracked())}</Text> }
    })
}

/// Renders the interactive theme switching demo.
///
/// # Returns
///
/// A [`View`] containing themed content and toggle controls.
#[component]
fn ThemeDemo() -> impl IntoView {
    let mode = RwSignal::new(ThemeMode::Light);
    let theme = RwSignal::new(ThemeMode::Light.variables());

    provide_context(mode.read_only());
    provide_context(theme.read_only());

    stylesheet! {
        $text: theme_color("text");
        $muted: theme_color("muted");
        $surface: theme_color("surface");
        $accent: theme_color("accent");
        $focus_text: theme_color("focus_text");
        $focus_surface: theme_color("focus_surface");

        .app-panel => {
            fg: $text,
            bg: $surface,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1),

            .title => { fg: $accent, modifier: Modifier::BOLD }
            .theme-status => { fg: $text }
            .body => { fg: $muted }
            .theme-button => { fg: $text, bg: $surface }
            .danger => { fg: Color::LightRed }

            Button => {
                &:focus => {
                    fg: $focus_text,
                    bg: $focus_surface,
                    modifier: Modifier::BOLD,
                    border_type: BorderType::Thick
                }
            }
        }

        .controls => { display: Display::Flex }

        @media (max-width: 60) {
            .app-panel => { padding: TuiSpacing::ZERO }
            .controls => { flex_direction: FlexDirection::Column }
            .theme-button => {
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! {
        <Block class="app-panel">
            <Div>
                <Text class="title">"Theme variables"</Text>
                <ThemeStatus />
                <Text class="body">
                    "The same stylesheet resolves against the active context theme."
                </Text>
                <Div class="controls">
                    <Button
                        class="theme-button"
                        on_press=move || {
                            mode.update(|mode| {
                                *mode = mode.toggle();
                                theme.set(mode.variables());
                            });
                            AppControl::Continue
                        }
                    >
                        "Toggle theme"
                    </Button>
                    <Button class="theme-button danger" on_press=|| AppControl::Exit>
                        "Quit"
                    </Button>
                </Div>
            </Div>
        </Block>
    }
}

/// Runs the theme switching example.
///
/// # Returns
///
/// An empty [`Result`] on successful app shutdown.
///
/// # Errors
///
/// Returns [`Error`] if the terminal app fails to initialize, render, or
/// process events.
#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    let view = view! { <ThemeDemo /> };
    App::new(view).run().await
}
