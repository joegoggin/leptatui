//! Fail fixture for literal `on_press` callback values.

use leptatui::prelude::*;

fn main() {
    let _view: View = view! {
        <Button on_press="save">"Save"</Button>
    };
}
