//! Fail fixture for duplicate `Image` source attributes.

use leptatui::prelude::*;

/// Triggers the `Image` duplicate-source validation failure.
fn main() {
    let _ = view! {
        <Image src="one.png" src="two.png" />
    };
}
