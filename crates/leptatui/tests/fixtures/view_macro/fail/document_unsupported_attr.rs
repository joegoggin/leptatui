//! Fail fixture for a document attribute on the wrong element.

use leptatui::prelude::*;

/// Triggers document attribute ownership validation.
fn main() {
    let _ = view! {
        <Paragraph language="rust">"Not code"</Paragraph>
    };
}
