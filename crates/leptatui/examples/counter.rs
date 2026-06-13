//! Interactive counter example.
//!
//! This binary demonstrates Leptos signals, Leptatui node rendering, stylesheet
//! focus rules, and button activation through the application runner.

use leptatui::prelude::*;

/// Root component for the interactive counter example.
#[component]
fn Counter() -> Node {
    let (count, set_count) = signal(0);
    let increment = set_count;
    let decrement = set_count;
    let reset = set_count;

    view! {
        <Column>
            <Block class="counter-panel">
                {move || {
                    view! {
                        <Text class="counter-value">{format!("Count: {}", count.get_untracked())}</Text>
                    }
                }}
            </Block>
            <Row class="counter-controls">
                <Button
                    class="counter-button"
                    on_press={move || {
                        increment.update(|count| *count += 1);
                        AppControl::Continue
                    }}
                >
                    "Increment"
                </Button>
                <Button
                    class="counter-button"
                    on_press={move || {
                        decrement.update(|count| *count -= 1);
                        AppControl::Continue
                    }}
                >
                    "Decrement"
                </Button>
                <Button
                    class="counter-button"
                    on_press={move || {
                        reset.set(0);
                        AppControl::Continue
                    }}
                >
                    "Reset"
                </Button>
                <Button class="counter-button danger" on_press={|| AppControl::Exit}>
                    "Quit"
                </Button>
            </Row>
            <Text class="counter-help">"Tab/Shift+Tab move focus. Enter/Space activate."</Text>
        </Column>
    }
}

/// Runs the counter example application.
#[tokio::main]
async fn main() -> Result<()> {
    let stylesheet = stylesheet! {
        Button => {
            fg: Color::White,
            borders: Borders::ALL,
            border_type: BorderType::Rounded
        }
        .danger => { fg: Color::LightRed }
        :focus => {
            fg: Color::Black,
            bg: Color::Yellow,
            modifier: Modifier::BOLD,
            border_type: BorderType::Thick
        }
    };

    App::new(Counter::new())
        .with_stylesheet(stylesheet)
        .run()
        .await
}
