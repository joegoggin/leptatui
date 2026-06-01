use leptatui::{Node, block, text};
use leptatui_macros::view;

fn main() {
    let node: Node = view! {
        <Block class="card" id="main" style={leptatui::TuiStyle::new()}>
            <Text>"hello"</Text>
        </Block>
    };

    assert_eq!(node, block(text("hello")));
}
