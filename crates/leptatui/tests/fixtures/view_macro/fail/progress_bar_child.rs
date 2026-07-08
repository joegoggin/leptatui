//! Fail fixture for unsupported `ProgressBar` children.

use leptatui::prelude::*;

/// Triggers the `ProgressBar` child validation failure.
fn main() {
    let _ = view! {
        <ProgressBar value={0.5}>
            "bad"
        </ProgressBar>
    };
}
