//! Fail fixture for literal `TextArea` callbacks.

use leptatui::prelude::*;

/// Triggers the `on_input` callback validation failure.
fn main() {
    let _ = view! {
        <TextArea value="Ada" on_input="save" />
    };
}
