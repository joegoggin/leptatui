//! Fail fixture for data props that omit their reactive handle type.

use leptatui::prelude::*;

/// Rejects an implicit snapshot in a data prop declaration.
#[component]
fn Snapshot(label: String) -> impl IntoView {
    text(label)
}

/// Triggers component signature validation.
fn main() {}
