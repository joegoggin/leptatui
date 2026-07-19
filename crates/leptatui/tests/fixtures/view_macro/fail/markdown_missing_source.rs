//! Fail fixture for a Markdown tag without in-memory source.

use leptatui::prelude::*;

/// Triggers the `Markdown` missing-source validation failure.
fn main() {
    let _ = view! {
        <Markdown />
    };
}
