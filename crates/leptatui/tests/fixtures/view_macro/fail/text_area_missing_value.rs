//! Fail fixture for `TextArea` without a value attribute.

use leptatui::prelude::*;

/// Triggers the required `TextArea` value validation failure.
fn main() {
    let _ = view! {
        <TextArea placeholder="Notes" />
    };
}
