//! Fail fixture for the removed Markdown source attribute.

use leptatui::prelude::*;

/// Triggers the `Markdown` unsupported-source validation failure.
fn main() {
    let _ = view! {
        <Markdown source="guide.md" />
    };
}
