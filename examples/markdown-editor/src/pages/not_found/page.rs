//! Not Found route-level component.

use leptatui::prelude::*;

/// Renders an unmatched Markdown editor location.
///
/// # Returns
///
/// A not-found page component with a Home anchor.
#[component]
pub(crate) fn NotFoundPage() -> impl IntoView {
    let location = use_location();

    stylesheet! {
        .not-found-page => {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )

            &__title => {
                fg: Color::LightCyan,
                modifier: Modifier::BOLD
            }
            &__error => { fg: Color::LightRed }
        }
    }

    view! {
        <Div class="not-found-page">
            <Text class="not-found-page__title">"Page not found"</Text>
            <Text class="not-found-page__error">
                {move || format!("No page matches {}", location.pathname().get_untracked())}
            </Text>
            <A href="/" exact=true>
                "Return home"
            </A>
        </Div>
    }
}
