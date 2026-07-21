//! Fail fixture for duplicate ordered-list start attributes.

use leptatui::prelude::*;

/// Triggers duplicate document-attribute validation.
fn main() {
    let _ = view! {
        <OrderedList start=1 start={2}>
            <ListItem><Paragraph>"Duplicate"</Paragraph></ListItem>
        </OrderedList>
    };
}
