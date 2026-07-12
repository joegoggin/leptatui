//! Fail fixture for a list item outside a semantic list.

use leptatui::prelude::*;

/// Triggers direct list-item ancestry validation.
fn main() {
    let _: View = view! {
        <Column>
            <ListItem><Paragraph>"Orphan"</Paragraph></ListItem>
        </Column>
    };
}
