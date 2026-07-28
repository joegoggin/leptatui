//! Fallback page for unmatched Markdown editor routes.

use leptatui::prelude::*;

/// Renders an unmatched Markdown editor location.
///
/// # Returns
///
/// A not-found page component with a Home anchor.
#[component]
pub(super) fn NotFoundPage() -> impl IntoView {
    let location = use_location();
    view! {
        <Div class="page">
            <Text class="page-title">"Page not found"</Text>
            <Text class="error">
                {move || format!("No page matches {}", location.pathname().get())}
            </Text>
            <A href="/" exact=true>
                "Return home"
            </A>
        </Div>
    }
}
