//! Home page stylesheet registration.

use leptatui::prelude::*;

/// Registers the Home page stylesheet with the active component.
pub(super) fn use_home_page_styles() {
    stylesheet! {
        .home-page => {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )

            @media (max-width: 60) {
                Button => { padding: TuiSpacing::ZERO }
            }

            &__title => {
                fg: Color::LightCyan,
                modifier: Modifier::BOLD
            }

            &__path => { fg: Color::LightGreen }

            &__actions => {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                gap: Axes::new(Length::cells(1.0), Length::cells(0.0))

                @media (max-width: 60) {
                    flex_direction: FlexDirection::Column
                }
            }

            &__content => {
                flex_basis: Dimension::from(Length::cells(0.0)),
                flex_grow: 1.0,
                borders: Borders::ALL,
                padding: TuiSpacing::horizontal(1),
                overflow: Axes::new(Overflow::Hidden, Overflow::Auto)

                @media (max-width: 60) {
                    padding: TuiSpacing::ZERO
                }
            }

            &__help => { fg: Color::Gray }
        }
    };
}
