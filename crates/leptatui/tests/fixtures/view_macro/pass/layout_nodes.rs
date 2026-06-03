//! Pass fixture for nested layout nodes in `view!`.
//!
//! This binary verifies columns, rows, text nodes, and buttons compose into the
//! expected render-tree shape.

use leptatui::prelude::*;

/// Exercises nested row and column expansion.
fn main() {
    let node: Node = view! {
        <Column>
            <Text>{"Counter"}</Text>
            <Row>
                <Button>"Increment"</Button>
                <Button>{"Reset"}</Button>
            </Row>
        </Column>
    };

    assert_eq!(
        node,
        column([text("Counter"), row([button("Increment"), button("Reset")]),])
    );
}
