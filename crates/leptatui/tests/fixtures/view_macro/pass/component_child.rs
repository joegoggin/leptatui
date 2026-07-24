//! Pass fixture for component children in `view!`.
//!
//! This binary verifies a braced component constructor lowers into a component
//! view when used inside a layout element.

use leptatui::prelude::*;

/// Builds a component used as a braced child expression.
#[component]
fn Label() -> impl IntoView {
    view! {
        <Text>"Count"</Text>
    }
}

/// Exercises component child expansion within a column.
fn main() {
    let view = view! {
        <Div>
            {Label::new()}
            <Text>"Help"</Text>
        </Div>
    };

    assert_eq!(view.metadata().view_type(), ViewType::Div);
    assert_eq!(view.children().len(), 2);
    assert!(view.children()[0].style_metadata().is_none());
    assert_eq!(view.children()[1], text("Help"));
}
