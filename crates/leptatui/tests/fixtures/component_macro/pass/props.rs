//! Pass fixture for `#[component]` props.
//!
//! This binary verifies generated props structs, builders, defaults, `into`
//! setters, and children prop types compile.

use leptatui::prelude::*;

/// Builds a text node from a required prop.
#[component]
fn RequiredLabel(#[prop(into)] label: String) -> Node {
    text(label)
}

/// Builds a text node from optional and defaulted props.
#[component]
fn DefaultedLabel(
    #[prop(optional, into)] prefix: String,
    #[prop(default = "fallback".to_string(), into)] label: String,
) -> Node {
    text(format!("{prefix}{label}"))
}

/// Builds a column from nested children.
#[component]
fn ChildPanel(#[prop(into)] title: String, children: Children) -> Node {
    column([text(title), column(children())])
}

/// Exercises generated prop builders and constructors.
fn main() {
    let _required = RequiredLabel::with_props(RequiredLabelProps::builder().label("Count").build());
    let _defaulted = DefaultedLabel::new();
    let _overridden = DefaultedLabel::with_props(
        DefaultedLabelProps::builder()
            .prefix("Active: ")
            .label("theme")
            .build(),
    );
    let _children = ChildPanel::with_props(
        ChildPanelProps::builder()
            .title("Panel")
            .children(Box::new(|| vec![text("child")]))
            .build(),
    );
}
