//! Fail fixture for unsupported Markdown children.

use leptatui::prelude::*;

/// Triggers the `Markdown` child validation failure.
fn main() {
    let _ = view! {
        <Markdown source="# Guide">
            "bad"
        </Markdown>
    };
}
