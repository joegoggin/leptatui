//! Fail fixture for duplicate component children sources.
//!
//! This binary triggers the diagnostic for supplying both a `children` prop and
//! nested component tag content.

use leptatui::prelude::*;

/// Accepts children from either a prop or nested content.
#[component]
fn Panel(children: Children) -> View {
    column(children())
}

/// Triggers the duplicate children diagnostic.
fn main() {
    let _ = view! {
        <Panel children={Box::new(|| vec![text("prop")])}>
            <Text>"nested"</Text>
        </Panel>
    };
}
