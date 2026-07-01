//! Fail fixture for literal `Input` callbacks.

use leptatui::prelude::*;

/// Triggers the `on_input` callback validation failure.
fn main() {
    let _ = view! {
        <Input value="Ada" on_input="save" />
    };
}
