//! Minimal app runner smoke example.
//!
//! This binary renders a small static node tree and exits from the Quit button.

use leptatui::prelude::*;

/// Root component for the smoke example.
#[component]
fn Root() -> Node {
    view! {
        <Block>
            <Column>
                <Text>"Leptatui smoke runner. Focus Quit and press Enter or Space."</Text>
                <Button on_press={|| AppControl::Exit}>"Quit"</Button>
            </Column>
        </Block>
    }
}

/// Runs the smoke example application.
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
