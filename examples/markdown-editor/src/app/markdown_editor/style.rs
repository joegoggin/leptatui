//! Markdown editor shell stylesheet registration.

use leptatui::prelude::*;

/// Registers the Markdown editor shell stylesheet with the active component.
pub(super) fn use_markdown_editor_styles() {
    stylesheet! {
        .markdown-editor => {
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
