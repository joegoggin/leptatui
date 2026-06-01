use crossterm::event::{Event, KeyCode, KeyEventKind};
use leptatui::prelude::*;
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};

struct Counter {
    _owner: Owner,
    count: ReadSignal<i32>,
    set_count: WriteSignal<i32>,
    theme: CounterTheme,
}

impl Counter {
    fn new() -> Self {
        let owner = Owner::new();
        let (count, set_count) = owner.with(|| signal(0));

        Self {
            _owner: owner,
            count,
            set_count,
            theme: CounterTheme::default(),
        }
    }
}

impl Component for Counter {
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let areas = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(ctx.area());

        ctx.with_area(areas[0], |ctx| {
            ctx.render_widget(
                Paragraph::new("Leptatui counter")
                    .centered()
                    .style(self.theme.heading.to_ratatui_style())
                    .block(self.theme.panel.to_block().title("Demo")),
            );
        });

        ctx.with_area(areas[1], |ctx| {
            ctx.render_widget(
                Paragraph::new(format!("Count: {}", self.count.get_untracked()))
                    .centered()
                    .style(self.theme.value.to_ratatui_style())
                    .block(self.theme.value_panel.to_block()),
            );
        });

        ctx.with_area(areas[2], |ctx| {
            ctx.render_widget(
                Paragraph::new("+/Up increment  -/Down decrement  0 reset  q/Esc quit")
                    .centered()
                    .style(self.theme.help.to_ratatui_style()),
            );
        });

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        let Event::Key(key) = event else {
            return Ok(AppControl::Continue);
        };

        if key.kind != KeyEventKind::Press {
            return Ok(AppControl::Continue);
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Ok(AppControl::Exit),
            KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Up => {
                self.set_count.update(|count| *count += 1);
                Ok(AppControl::Continue)
            }
            KeyCode::Char('-') | KeyCode::Down => {
                self.set_count.update(|count| *count -= 1);
                Ok(AppControl::Continue)
            }
            KeyCode::Char('0') => {
                self.set_count.set(0);
                Ok(AppControl::Continue)
            }
            _ => Ok(AppControl::Continue),
        }
    }
}

#[derive(Clone, Copy)]
struct CounterTheme {
    panel: TuiStyle,
    value_panel: TuiStyle,
    heading: TuiStyle,
    value: TuiStyle,
    help: TuiStyle,
}

impl Default for CounterTheme {
    fn default() -> Self {
        let base = TuiStyle::new().background(Color::Black);

        Self {
            panel: base
                .foreground(Color::LightCyan)
                .modifier(Modifier::BOLD)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(TuiSpacing::horizontal(1)),
            value_panel: base
                .foreground(Color::Yellow)
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .padding(TuiSpacing::uniform(1)),
            heading: base.foreground(Color::White).modifier(Modifier::BOLD),
            value: base.foreground(Color::LightGreen).modifier(Modifier::BOLD),
            help: base.foreground(Color::Gray),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let root = Counter::new();
    App::new(root).run().await
}
