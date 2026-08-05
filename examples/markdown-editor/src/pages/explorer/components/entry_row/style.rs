//! Explorer entry-row stylesheet registration.

use leptatui::prelude::*;

/// Registers the Explorer entry-row stylesheet with the active component.
pub(super) fn use_explorer_entry_row_styles() {
    stylesheet! {
        .explorer-entry => {
            &--directory => { fg: Color::LightBlue }
            &--markdown => { fg: Color::White }
            &--selected => {
                fg: Color::Black,
                bg: Color::LightCyan,
                modifier: Modifier::BOLD
            }
        }
    };
}
