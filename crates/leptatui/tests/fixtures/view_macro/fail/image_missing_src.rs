//! Fail fixture for missing `Image` source attributes.

use leptatui::prelude::*;

/// Triggers the `Image` missing-source validation failure.
fn main() {
    let _ = view! {
        <Image alt="Project logo" />
    };
}
