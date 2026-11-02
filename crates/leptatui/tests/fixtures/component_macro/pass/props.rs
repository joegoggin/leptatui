//! Pass fixture for `#[component]` props.
//!
//! This binary verifies generated props structs, builders, defaults, `into`
//! setters, and children prop types compile.

use leptatui::prelude::*;

/// Builds a text view from a required prop.
#[component]
fn RequiredLabel(#[prop(into)] label: leptatui::Prop<String>) -> impl IntoView {
    view! { <Text>{label.get()}</Text> }
}

/// Builds a text view from optional and defaulted props.
#[component]
fn DefaultedLabel(
    #[prop(optional, into)] prefix: leptatui::Prop<String>,
    #[prop(default = "fallback".to_string(), into)] label: leptatui::Prop<String>,
) -> impl IntoView {
    view! { <Text>{format!("{}{}", prefix.get(), label.get())}</Text> }
}

/// Builds a column from nested children.
#[component]
fn ChildPanel(#[prop(into)] title: leptatui::Prop<String>, children: Children) -> impl IntoView {
    view! { <Div><Text>{title.get()}</Text>{children()}</Div> }
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
            .children(Box::new(|| vec![text("child").into_view()]))
            .build(),
    );
}
