//! Hello World example.
//!
//! This binary renders a small static node tree and exits when `q` is pressed.

use leptatui::prelude::*;

/// Root component for the Hello World example.
#[component]
fn Root() -> Node {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    view! {
        <Block>
            <Column>
                <Text>"Hello, world!"</Text>
                <Text>"Press q to quit."</Text>
            </Column>
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
    App::new(Root::new()).run().await
}
