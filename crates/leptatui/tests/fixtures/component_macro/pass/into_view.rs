//! Pass fixture for converting generated components into views.
//!
//! This binary verifies generated component values can cross the view boundary
//! through [`Into`] conversion.

use leptatui::prelude::*;

/// Returns a view built from `view!` syntax.
#[component]
fn Greeting() -> impl IntoView {
    view! {
        <Text>"hello"</Text>
    }
}

/// Exercises conversion from a generated component into [`View`].
fn main() {
    let _view: AnyView = Greeting::new().into_view();
}
