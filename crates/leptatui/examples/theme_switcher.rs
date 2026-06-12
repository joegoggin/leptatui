//! Context-backed light/dark theme switching example.

use crossterm::event::Event;
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

struct ThemeStatus;

impl Component for ThemeStatus {
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let mode = expect_context::<ReadSignal<ThemeMode>>().get_untracked();
        ctx.render_node(&text(format!("Active theme: {mode:?}")).with_classes("theme-status"))
    }
}

struct ThemeDemo {
    _owner: Owner,
    mode: ReadSignal<ThemeMode>,
    root: Node,
}

impl ThemeDemo {
    fn new() -> Self {
        let owner = Owner::new();
        let (mode, set_mode) = owner.with(|| signal(ThemeMode::Light));
        let toggle = set_mode;
        let root = block(column([
            text("Theme variables").with_classes("title"),
            component(ThemeStatus),
            text("The same stylesheet resolves against the active context theme.")
                .with_classes("body"),
            row([
                button("Toggle theme")
                    .with_classes("theme-button")
                    .on_press(move || {
                        toggle.update(|mode| *mode = mode.toggle());
                        AppControl::Continue
                    }),
                button("Quit")
                    .with_classes("theme-button danger")
                    .on_press(|| AppControl::Exit),
            ])
            .with_classes("controls"),
        ]))
        .with_classes("app-panel");

        Self {
            _owner: owner,
            mode,
            root,
        }
    }
}

impl Component for ThemeDemo {
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let mode = self.mode.get_untracked();
        provide_context(mode.variables());
        provide_context(self.mode);
        ctx.render_node(&self.root)
    }

    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        self.root.handle_event(event)
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
