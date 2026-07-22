//! Interactive counter example.
//!
//! This binary demonstrates Leptos signals, Leptatui view rendering, stylesheet
//! rules, and explicit key maps through the application runner.

use leptatui::prelude::*;

/// Root component for the interactive counter example.
#[component]
fn Counter() -> impl IntoView {
    let count = RwSignal::new(0);

    use_key_event(KeyEventKind::Press, move |key| match key.code {
        KeyCode::Char('+') | KeyCode::Char('=') => {
            count.update(|count| *count += 1);
            KeyControl::Handled
        }
        KeyCode::Char('-') => {
            count.update(|count| *count -= 1);
            KeyControl::Handled
        }
        KeyCode::Char('r') => {
            count.set(0);
            KeyControl::Handled
        }
        KeyCode::Char('q') => KeyControl::Exit,
        _ => KeyControl::Pass,
    });

    stylesheet! {
        .counter-panel => {
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }
        .counter-value => { fg: Color::LightCyan, modifier: Modifier::BOLD }
        .counter-help => { fg: Color::Gray }
        Button => {
            fg: Color::White,
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)
        }
        .danger => { fg: Color::LightRed }

        @media (max-width: 60) {
            .counter-panel => { padding: TuiSpacing::ZERO }
            .counter-controls => { direction: LayoutDirection::Column }
            .counter-button => {
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! {
        <Column>
            <Block class="counter-panel">
                {move || {
                    view! {
                        <Text class="counter-value">
                            {format!("Count: {}", count.get_untracked())}
                        </Text>
                    }
                }}
            </Block>
            <Row class="counter-controls">
                <Button class="counter-button">"+ Increment"</Button>
                <Button class="counter-button">"- Decrement"</Button>
                <Button class="counter-button">"r Reset"</Button>
                <Button class="counter-button danger">"q Quit"</Button>
            </Row>
            <Text class="counter-help">"+/- adjust. r resets. q quits."</Text>
        </Column>
    }
}

/// Runs the counter example application.
#[tokio::main]
async fn main() -> Result<()> {
    App::new(Counter::new()).run().await
}
