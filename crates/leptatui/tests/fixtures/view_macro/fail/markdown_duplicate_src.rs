//! Fail fixture for duplicate Markdown src attributes.

use leptatui::prelude::*;

/// Triggers the `Markdown` duplicate-src validation failure.
fn main() {
    let _: View = view! {
        <Markdown src="one.md" src="two.md" />
    };
}
