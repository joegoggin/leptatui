//! Pass fixture for Leptos-style component tags in `view!`.
//!
//! This binary verifies component tags lower through generated props builders,
//! including `into` props, unbraced closure props, and nested children.

use leptatui::prelude::*;

/// Builds a text view from a prop.
#[component]
fn Label(#[prop(into)] text: String) -> impl IntoView {
    view! { <Text>{text}</Text> }
}

/// Builds a button from an unbraced callback prop.
#[component]
fn ActionLabel(on_press: fn() -> AppControl) -> impl IntoView {
    view! { <Button on_press={on_press}>"Run"</Button> }
}

/// Builds a panel around nested children.
#[component]
fn Panel(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <Column>
            <Text>{title}</Text>
            {column(children())}
        </Column>
    }
}

/// Exercises component tags at root and child positions.
fn main() {
    let root = view! { <Label text="Count" /> };
    assert!(root.style_metadata().is_none());

    let view = view! {
        <Panel title="Theme variables">
            <Label text="Active theme" />
            <ActionLabel on_press=|| AppControl::Continue />
            "Loose text"
        </Panel>
    };

    assert!(view.style_metadata().is_none());
}
