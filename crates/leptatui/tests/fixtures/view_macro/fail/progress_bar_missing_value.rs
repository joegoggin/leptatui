//! Fail fixture for `ProgressBar` without a value attribute.

use leptatui::prelude::*;

/// Triggers the required `ProgressBar` value validation failure.
fn main() {
    let _ = view! {
        <ProgressBar label="Uploading" />
    };
}
