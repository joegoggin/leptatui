//! Interactive counter example.
//!
//! This binary demonstrates Leptos signals, Leptatui styling helpers, component
//! rendering, and keyboard event handling in a terminal app.

use crossterm::event::{Event, KeyCode, KeyEventKind};
use leptatui::prelude::*;
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};

/// Root component for the interactive counter example.
struct Counter {
    /// Leptos owner that keeps the component's reactive graph alive.
    _owner: Owner,
    /// Current counter value.
    count: ReadSignal<i32>,
    /// Setter used to mutate the counter value.
    set_count: WriteSignal<i32>,
    /// Style values used while rendering the counter UI.
    theme: CounterTheme,
}

impl Counter {
    /// Creates a counter component with a zero value.
    ///
    /// # Returns
    ///
    /// A [`Counter`] initialized with its own Leptos owner and default theme.
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
    /// Renders the current counter value and keyboard help.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context for the current frame.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
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

    /// Handles keyboard input for counter updates and app exit.
    ///
    /// # Arguments
    ///
    /// * `event` — Terminal event emitted by Crossterm.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether to continue or exit.
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

/// Style bundle for the counter example.
#[derive(Clone, Copy)]
struct CounterTheme {
    /// Outer panel style used by the heading.
    panel: TuiStyle,
    /// Panel style used by the current value display.
    value_panel: TuiStyle,
    /// Text style used by the heading.
    heading: TuiStyle,
    /// Text style used by the current value.
    value: TuiStyle,
    /// Text style used by the keyboard help.
    help: TuiStyle,
}

impl Default for CounterTheme {
    /// Creates the default counter theme.
    ///
    /// # Returns
    ///
    /// A [`CounterTheme`] with contrasting panel, value, heading, and help
    /// styles.
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

/// Runs the counter example application.
///
/// # Returns
///
/// An empty [`Result`] when the app exits successfully.
///
/// # Errors
///
/// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup fails.
/// Returns [`Error::EventTask`] if the blocking event task fails.
#[tokio::main]
async fn main() -> Result<()> {
    let root = Counter::new();
    App::new(root).run().await
}
