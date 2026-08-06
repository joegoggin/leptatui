//! Viewer document stylesheet registration.

use leptatui::prelude::*;

/// Registers the Viewer document stylesheet with the active component.
pub(super) fn use_viewer_document_styles() {
    stylesheet! {
        .viewer-document => {
            flex_basis: Dimension::from(Length::cells(0.0)),
            flex_grow: 1.0,
            borders: Borders::ALL,
            padding: TuiSpacing::horizontal(1),
            overflow: Axes::new(Overflow::Hidden, Overflow::Auto)

            @media (max-width: 60) {
                padding: TuiSpacing::ZERO
            }

            &__empty => { fg: Color::DarkGray }
        }
    };
}
