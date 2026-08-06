//! Not Found route-level component.

use leptatui::prelude::*;

use super::style::use_not_found_page_styles;

/// Renders an unmatched Markdown editor location.
///
/// # Returns
///
/// A not-found page component with a Home anchor.
#[component]
pub(crate) fn NotFoundPage() -> impl IntoView {
    let location = use_location();

    use_not_found_page_styles();

    view! {
        <Div class="not-found-page">
            <Text class="not-found-page__title">"Page not found"</Text>
            <Text class="not-found-page__error">
                {move || format!("No page matches {}", location.pathname().get())}
            </Text>
            <A href="/" exact=true>
                "Return home"
            </A>
        </Div>
    }
}
