//! Fail fixture for trailing tokens in braced `view!` content.
//!
//! This binary triggers the diagnostic for braced content that contains more
//! than one Rust expression.

use leptatui::prelude::*;

/// Triggers a braced-expression parse failure.
fn main() {
    let label = String::from("bad");

    let _ = view! {
        <Text>{label extra}</Text>
    };
}
