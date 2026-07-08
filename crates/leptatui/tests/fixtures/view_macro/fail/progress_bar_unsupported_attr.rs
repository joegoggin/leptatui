//! Fail fixture for unsupported `ProgressBar` attributes.

use leptatui::prelude::*;

/// Triggers the `ProgressBar` unsupported-attribute validation failure.
fn main() {
    let _ = view! {
        <ProgressBar value={0.5} role="meter" />
    };
}
