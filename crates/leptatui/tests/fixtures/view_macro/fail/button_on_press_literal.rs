//! Fail fixture for literal `on_press` callback values.

use leptatui::prelude::*;

/// Exercises rejection of literal `on_press` callback values.
fn main() {
    let _view: View = view! {
        <Button on_press="save">"Save"</Button>
    };
}
