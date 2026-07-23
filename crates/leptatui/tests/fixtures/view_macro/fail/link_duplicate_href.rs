//! Fail fixture for duplicate `Link` destinations.

use leptatui::prelude::*;

/// Triggers the `Link` duplicate-href validation failure.
fn main() {
    let _ = view! {
        <Link href="one.md" href="two.md">"Guide"</Link>
    };
}
