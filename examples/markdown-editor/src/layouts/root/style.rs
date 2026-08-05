//! Root layout stylesheet registration.

use leptatui::prelude::*;

/// Registers the root layout stylesheet with the active component.
pub(super) fn use_root_layout_styles() {
    stylesheet! {
        .root-layout => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1),
            box_sizing: BoxSizing::BorderBox,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )

            @media (max-width: 60) {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }

            &__routes => {
                position: Position::Relative,
                size: LayoutSize::new(
                    Dimension::from(Length::percent(100.0)),
                    Dimension::from(Length::percent(100.0))
                )
            }
        }
    };
}
