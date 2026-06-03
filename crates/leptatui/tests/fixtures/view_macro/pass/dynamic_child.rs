//! Pass fixture for dynamic children in `view!`.
//!
//! This binary verifies closure child expressions become deferred dynamic nodes
//! while static sibling elements still expand normally.

use leptatui::prelude::*;

/// Exercises dynamic child expansion within a column.
fn main() {
    let count = 7;

    let node: Node = view! {
        <Column>
            {move || text(count.to_string())}
            <Text>"Static"</Text>
        </Column>
    };

    assert!(matches!(
        node,
        Node::Column { children, .. }
            if matches!(children.first(), Some(Node::Dynamic(_)))
                && children.get(1) == Some(&text("Static"))
    ));
}
