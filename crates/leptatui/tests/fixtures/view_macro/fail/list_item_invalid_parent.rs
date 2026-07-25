//! Fail fixture for a list item outside a semantic list.

use leptatui::prelude::*;

/// Triggers direct list-item ancestry validation.
fn main() {
    let _ = view! {
        <Div>
            <ListItem><Paragraph>"Orphan"</Paragraph></ListItem>
        </Div>
    };
}
