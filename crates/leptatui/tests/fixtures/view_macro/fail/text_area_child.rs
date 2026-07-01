//! Fail fixture for unsupported `TextArea` children.

use leptatui::prelude::*;

/// Triggers the `TextArea` child validation failure.
fn main() {
    let _ = view! {
        <TextArea value="Ada">
            "bad"
        </TextArea>
    };
}
