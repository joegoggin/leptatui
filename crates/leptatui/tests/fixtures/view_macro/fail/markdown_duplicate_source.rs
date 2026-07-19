//! Fail fixture for duplicate Markdown source attributes.

use leptatui::prelude::*;

/// Triggers the `Markdown` duplicate-source validation failure.
fn main() {
    let _ = view! {
        <Markdown source="# One" source="# Two" />
    };
}
