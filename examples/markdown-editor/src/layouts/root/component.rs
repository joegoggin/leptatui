//! Root application frame and child-content boundary.

use leptatui::prelude::*;

use super::style::use_root_layout_styles;

/// Renders application children inside the shared root frame.
///
/// # Arguments
///
/// * `children` — Routed content and application overlays.
///
/// # Returns
///
/// A root layout containing the supplied children.
#[component]
pub(crate) fn RootLayout(children: Children) -> impl IntoView {
    use_root_layout_styles();

    view! {
        <Block class="root-layout">
            <Div class="root-layout__routes">{children()}</Div>
        </Block>
    }
}
