use leptatui::{Node, text};
use leptatui_macros::view;

fn main() {
    let count = 7;

    let node: Node = view! { <Text>{move || count.to_string()}</Text> };

    assert_eq!(node, text("7"));
}
