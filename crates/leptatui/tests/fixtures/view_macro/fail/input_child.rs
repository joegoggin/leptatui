//! Fail fixture for unsupported `Input` children.

use leptatui::prelude::*;

/// Triggers the `Input` child validation failure.
fn main() {
    let _ = view! {
        <Input value="Ada">
            "bad"
        </Input>
    };
}
