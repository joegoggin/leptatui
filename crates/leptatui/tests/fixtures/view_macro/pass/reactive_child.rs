//! Pass fixture for reactive text children in `view!`.
//!
//! This binary verifies a closure expression inside text content is invoked
//! during macro expansion into a text view.

use leptatui::prelude::*;

/// Exercises closure text content expansion.
fn main() {
    let count = 7;

    let view = view! { <Text>{move || count.to_string()}</Text> };

    assert_eq!(view, text("7"));
}
