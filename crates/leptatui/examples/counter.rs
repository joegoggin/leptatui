//! Interactive counter example.
//!
//! This binary demonstrates Leptos signals, Leptatui node rendering, stylesheet
//! focus rules, and button activation through the application runner.

use crossterm::event::Event;
use leptatui::prelude::*;

/// Root component for the interactive counter example.
struct Counter {
    /// Leptos owner that keeps the component's reactive graph alive.
    _owner: Owner,
    /// Node tree that owns counter controls and dispatches focused events.
    root: Node,
}

impl Counter {
    /// Creates a counter component with a zero value.
    fn new() -> Self {
        let owner = Owner::new();
        let (count, set_count) = owner.with(|| signal(0));

        let increment = set_count;
        let decrement = set_count;
        let reset = set_count;

        let root = column([
            block(dynamic(move || {
                text(format!("Count: {}", count.get_untracked())).with_classes("counter-value")
            }))
            .with_classes("counter-panel"),
            row([
                button("Increment")
                    .with_classes("counter-button")
                    .on_press(move || {
                        increment.update(|count| *count += 1);
                        AppControl::Continue
                    }),
                button("Decrement")
                    .with_classes("counter-button")
                    .on_press(move || {
                        decrement.update(|count| *count -= 1);
                        AppControl::Continue
                    }),
                button("Reset")
                    .with_classes("counter-button")
                    .on_press(move || {
                        reset.set(0);
                        AppControl::Continue
                    }),
                button("Quit")
                    .with_classes("counter-button danger")
                    .on_press(|| AppControl::Exit),
            ])
            .with_classes("counter-controls"),
            text("Tab/Shift+Tab move focus. Enter/Space activate.").with_classes("counter-help"),
        ]);

        Self {
            _owner: owner,
            root,
        }
    }
}

impl Component for Counter {
    /// Renders the current counter node tree.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_node(&self.root)
    }

    /// Delegates terminal events to the node tree.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        self.root.handle_event(event)
    }
}

/// Runs the counter example application.
#[tokio::main]
async fn main() -> Result<()> {
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::node_type(NodeType::Button),
            TuiStyle::new()
                .foreground(Color::White)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .rule(
            StyleSelector::class("danger"),
            TuiStyle::new().foreground(Color::LightRed),
        )
        .rule(
            StyleSelector::focus(),
            TuiStyle::new()
                .foreground(Color::Black)
                .background(Color::Yellow)
                .modifier(Modifier::BOLD)
                .border_type(BorderType::Thick),
        );

    App::new(Counter::new())
        .with_stylesheet(stylesheet)
        .run()
        .await
}
