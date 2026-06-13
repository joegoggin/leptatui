//! Context-backed light/dark theme switching example.

use leptatui::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

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

#[component]
fn ThemeStatus() -> Node {
    let mode = expect_context::<ReadSignal<ThemeMode>>();

    dynamic(move || {
        view! {
            <Text class="theme-status">{format!("Active theme: {:?}", mode.get_untracked())}</Text>
        }
    })
}

#[component]
fn ThemeDemo() -> Node {
    let (mode, set_mode) = signal(ThemeMode::Light);
    let (theme, set_theme) = signal(ThemeMode::Light.variables());

    provide_context(mode);
    provide_context(theme);

    view! {
        <Block class="app-panel">
            <Column>
                <Text class="title">"Theme variables"</Text>
                {ThemeStatus::new()}
                <Text class="body">"The same stylesheet resolves against the active context theme."</Text>
                <Row class="controls">
                    <Button
                        class="theme-button"
                        on_press={move || {
                            let next = mode.get_untracked().toggle();
                            set_mode.set(next);
                            set_theme.set(next.variables());
                            AppControl::Continue
                        }}
                    >
                        "Toggle theme"
                    </Button>
                    <Button class="theme-button danger" on_press={|| AppControl::Exit}>
                        "Quit"
                    </Button>
                </Row>
            </Column>
        </Block>
    }
}

fn theme_stylesheet() -> Stylesheet {
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
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    App::new(ThemeDemo::new())
        .with_stylesheet(theme_stylesheet())
        .run()
        .await
}
