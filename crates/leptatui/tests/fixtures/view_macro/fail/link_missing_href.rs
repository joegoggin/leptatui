//! Fail fixture for a `Link` without a destination.

use leptatui::prelude::*;

/// Triggers the `Link` missing-href validation failure.
fn main() {
    let _ = view! {
        <Link>"Guide"</Link>
    };
}
