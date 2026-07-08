//! Fail fixture for duplicate `ProgressBar` value attributes.

use leptatui::prelude::*;

/// Triggers the `ProgressBar` duplicate-value validation failure.
fn main() {
    let _ = view! {
        <ProgressBar value={0.25} value={0.75} />
    };
}
