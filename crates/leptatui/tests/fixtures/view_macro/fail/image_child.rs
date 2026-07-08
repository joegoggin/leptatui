//! Fail fixture for unsupported `Image` children.

use leptatui::prelude::*;

/// Triggers the `Image` child validation failure.
fn main() {
    let _ = view! {
        <Image src="assets/logo.png">
            "bad"
        </Image>
    };
}
