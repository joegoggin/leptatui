//! Pass fixture for dynamic children in `view!`.
//!
//! This binary verifies closure child expressions become deferred dynamic views
//! while static sibling elements still expand normally.

use leptatui::prelude::*;

/// Exercises dynamic child expansion within a column.
fn main() {
    let count = 7;

    let view = view! {
        <Div>
            {move || text(count.to_string())}
            <Text>"Static"</Text>
        </Div>
    };

    assert_eq!(view.metadata().view_type(), ViewType::Div);
    assert!(view.children()[0].is::<DynamicView>());
    assert_eq!(view.children()[1], text("Static"));
}
