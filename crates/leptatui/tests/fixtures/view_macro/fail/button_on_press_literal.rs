//! Fail fixture for literal `on_press` callback values.

use leptatui::prelude::*;

fn main() {
    let _node: Node = view! {
        <Button on_press="save">"Save"</Button>
    };
}
