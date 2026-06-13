//! Pass fixture for nested layout views in `view!`.
//!
//! This binary verifies columns, rows, text views, and buttons compose into the
//! expected render-tree shape.

use leptatui::prelude::*;

/// Exercises nested row and column expansion.
fn main() {
    let view: View = view! {
        <Column>
            <Text>{"Counter"}</Text>
            <Row>
                <Button>"Increment"</Button>
                <Button>{"Reset"}</Button>
            </Row>
        </Column>
    };

    assert_eq!(
        view,
        column([text("Counter"), row([button("Increment"), button("Reset")]),])
    );
}
