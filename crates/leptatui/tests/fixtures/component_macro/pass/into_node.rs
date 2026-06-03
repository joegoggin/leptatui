//! Pass fixture for converting generated components into nodes.
//!
//! This binary verifies generated component values can cross the node boundary
//! through [`Into`] conversion.

use leptatui::prelude::*;

/// Returns a node built from `view!` syntax.
#[component]
fn Greeting() -> Node {
    view! {
        <Text>"hello"</Text>
    }
}

/// Exercises conversion from a generated component into [`Node`].
fn main() {
    let node: Node = Greeting::new().into();

    assert!(matches!(node, Node::Component(_)));
}
