use leptatui::prelude::*;

#[component]
fn Greeting() -> Node {
    view! {
        <Text>"hello"</Text>
    }
}

fn main() {
    let node: Node = Greeting::new().into();

    assert_eq!(node, text("hello"));
}
