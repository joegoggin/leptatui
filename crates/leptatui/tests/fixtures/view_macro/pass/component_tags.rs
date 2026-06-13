//! Pass fixture for Leptos-style component tags in `view!`.
//!
//! This binary verifies component tags lower through generated props builders,
//! including `into` props, unbraced closure props, and nested children.

use leptatui::prelude::*;

/// Builds a text node from a prop.
#[component]
fn Label(#[prop(into)] text: String) -> Node {
    view! { <Text>{text}</Text> }
}

/// Builds a button from an unbraced callback prop.
#[component]
fn ActionLabel(on_press: fn() -> AppControl) -> Node {
    view! { <Button on_press={on_press}>"Run"</Button> }
}

/// Builds a panel around nested children.
#[component]
fn Panel(#[prop(into)] title: String, children: Children) -> Node {
    view! {
        <Column>
            <Text>{title}</Text>
            {column(children())}
        </Column>
    }
}

/// Exercises component tags at root and child positions.
fn main() {
    let root: Node = view! { <Label text="Count" /> };
    assert!(matches!(root, Node::Component(_)));

    let node: Node = view! {
        <Panel title="Theme variables">
            <Label text="Active theme" />
            <ActionLabel on_press=|| AppControl::Continue />
            "Loose text"
        </Panel>
    };

    assert!(matches!(node, Node::Component(_)));
}
