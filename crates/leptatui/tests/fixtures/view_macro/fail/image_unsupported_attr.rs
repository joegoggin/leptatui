//! Fail fixture for unsupported `Image` attributes.

use leptatui::prelude::*;

/// Triggers the `Image` attribute validation failure.
fn main() {
    let _ = view! {
        <Image src="assets/logo.png" data_id="bad" />
    };
}
