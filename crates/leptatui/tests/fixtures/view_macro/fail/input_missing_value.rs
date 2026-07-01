//! Fail fixture for `Input` without a value attribute.

use leptatui::prelude::*;

/// Triggers the required `Input` value validation failure.
fn main() {
    let _ = view! {
        <Input placeholder="Name" />
    };
}
