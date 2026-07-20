//! Fail fixture for a string-valued code-block boolean attribute.

use leptatui::prelude::*;

/// Triggers typed code-block attribute validation.
fn main() {
    let _ = view! {
        <CodeBlock line_numbers="true">"let ready = true;"</CodeBlock>
    };
}
