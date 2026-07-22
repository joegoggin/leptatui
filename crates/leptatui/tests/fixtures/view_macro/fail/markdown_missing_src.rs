//! Fail fixture for a Markdown tag without a file path.

use leptatui::prelude::*;

/// Triggers the `Markdown` missing-src validation failure.
fn main() {
    let _ = view! {
        <Markdown />
    };
}
