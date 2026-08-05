//! Not Found page stylesheet registration.

use leptatui::prelude::*;

/// Registers the Not Found page stylesheet with the active component.
pub(super) fn use_not_found_page_styles() {
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
    };
}
