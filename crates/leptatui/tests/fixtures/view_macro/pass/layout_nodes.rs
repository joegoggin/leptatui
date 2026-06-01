use leptatui::{Node, button, column, row, text};
use leptatui_macros::view;

fn main() {
    let node: Node = view! {
        <Column>
            <Text>{"Counter"}</Text>
            <Row>
                <Button>"Increment"</Button>
                <Button>{"Reset"}</Button>
            </Row>
        </Column>
    };

    assert_eq!(
        node,
        column([text("Counter"), row([button("Increment"), button("Reset")]),])
    );
}
