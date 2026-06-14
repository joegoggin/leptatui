//! Pass fixture for component children in `view!`.
//!
//! This binary verifies a braced component constructor lowers into a component
//! view when used inside a layout element.

use leptatui::prelude::*;

/// Builds a component used as a braced child expression.
#[component]
fn Label() -> View {
    view! {
        <Text>"Count"</Text>
    }
}

/// Exercises component child expansion within a column.
fn main() {
    let view: View = view! {
        <Column>
            {Label::new()}
            <Text>"Help"</Text>
        </Column>
    };

    assert!(matches!(
        view,
        View::Column { children, .. }
            if matches!(children.first(), Some(View::Component(_)))
                && children.get(1) == Some(&text("Help"))
    ));
}
