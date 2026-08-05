//! Notification overlay stylesheet registration.

use leptatui::prelude::*;

/// Registers the notification overlay stylesheet with the active component.
pub(super) fn use_notifications_styles() {
    stylesheet! {
        .notifications => {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            position: Position::Fixed,
            inset: Edges::new(
                Length::cells(1.0).into(),
                Length::cells(1.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto
            ),
            z_index: ZIndex::Integer(10)

            &__item => {
                borders: Borders::ALL,
                border_type: BorderType::Rounded,
                padding: TuiSpacing::horizontal(1)

                &--success => { fg: Color::LightGreen }
                &--error => { fg: Color::LightRed }
                &--info => { fg: Color::LightCyan }
                &--warning => { fg: Color::Yellow }
            }
        }
    };
}
