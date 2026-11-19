//! Fail fixture for literal `TextArea` callbacks.

use leptatui::prelude::*;

/// Triggers the `on_input` callback validation failure.
fn main() {
    let _ = view! {
        <TextArea value={RwSignal::new(String::from("Ada"))} on_input="save" />
    };
}
