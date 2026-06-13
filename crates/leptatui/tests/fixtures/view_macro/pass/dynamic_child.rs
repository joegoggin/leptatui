//! Pass fixture for dynamic children in `view!`.
//!
//! This binary verifies closure child expressions become deferred dynamic views
//! while static sibling elements still expand normally.

use leptatui::prelude::*;

/// Exercises dynamic child expansion within a column.
fn main() {
    let count = 7;

    let view: View = view! {
        <Column>
            {move || text(count.to_string())}
            <Text>"Static"</Text>
        </Column>
    };

    assert!(matches!(
        view,
        View::Column { children, .. }
            if matches!(children.first(), Some(View::Dynamic(_)))
                && children.get(1) == Some(&text("Static"))
    ));
}
