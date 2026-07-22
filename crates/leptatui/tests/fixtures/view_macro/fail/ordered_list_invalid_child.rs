//! Fail fixture for a non-list-item ordered-list child.

use leptatui::prelude::*;

/// Triggers ordered-list child validation.
fn main() {
    let _ = view! {
        <OrderedList><Paragraph>"Not an item"</Paragraph></OrderedList>
    };
}
