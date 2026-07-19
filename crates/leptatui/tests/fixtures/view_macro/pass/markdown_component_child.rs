//! Pass fixture for infallible Markdown nested under a component tag.

use leptatui::prelude::*;

/// Renders deferred child views in a column.
///
/// # Arguments
///
/// * `children` — Deferred child views supplied by the parent macro invocation.
///
/// # Returns
///
/// A [`View`] containing the supplied children.
#[component]
fn Panel(children: Children) -> View {
    column(children())
}

/// Exercises path-backed Markdown fallback through component children.
fn main() {
    let view: View = view! {
        <Panel>
            <Markdown src="guide.md" />
        </Panel>
    };

    let _ = view;
}
