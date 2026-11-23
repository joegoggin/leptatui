//! Pass fixture for implicit reactive data props and retained inputs.

use leptatui::prelude::*;

/// Builds a heading from a data prop automatically wrapped in `Prop<String>`.
///
/// # Arguments
///
/// * `text` — Reactive heading content.
///
/// # Returns
///
/// A heading showing the current prop value.
#[component]
fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    view! { <H1>{text}</H1> }
}

/// Builds a heading from an explicitly retained prop handle.
///
/// # Arguments
///
/// * `text` — Prop handle retained without additional wrapping or conversion.
///
/// # Returns
///
/// A heading showing the current prop value.
#[component]
fn RetainedHeading(#[prop(static)] text: Prop<String>) -> impl IntoView {
    view! { <H1>{text}</H1> }
}

/// Exercises implicit prop storage and retained handle setters.
fn main() {
    let props = NoteHeadingProps::builder().text("My notes").build();
    let _: &Prop<String> = &props.text;
    let _heading = NoteHeading::with_props(props);
    let _reactive = view! { <NoteHeading text="My notes" /> };
    let _retained = RetainedHeading::with_props(
        RetainedHeadingProps::builder()
            .text(Prop::new(String::from("My notes")))
            .build(),
    );
}
