//! Hello World example.
//!
//! This binary renders a small static view tree and exits when `q` is pressed.

use leptatui::prelude::*;

/// Root component for the Hello World example.
#[component]
fn Root() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .hello-panel => {
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }

        @media (max-width: 60) {
            .hello-panel => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! {
        <Block class="hello-panel">
            <Div>
                <Text>"Hello, world!"</Text>
                <Text>"Press q to quit."</Text>
            </Div>
        </Block>
    }
}

/// Runs the Hello World example application.
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
    let view = view! { <Root /> };
    App::new(view).run().await
}
