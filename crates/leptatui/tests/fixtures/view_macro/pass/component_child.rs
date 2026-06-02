use leptatui::prelude::*;

#[component]
fn Label() -> Node {
    view! {
        <Text>"Count"</Text>
    }
}

fn main() {
    let node: Node = view! {
        <Column>
            {Label::new()}
            <Text>"Help"</Text>
        </Column>
    };

    assert_eq!(node, column([text("Count"), text("Help")]));
}
