//! Explorer list stylesheet registration.

use leptatui::prelude::*;

/// Registers the Explorer list stylesheet with the active component.
pub(super) fn use_explorer_list_styles() {
    stylesheet! {
        .explorer-list => {
            &__empty => { fg: Color::DarkGray }
            &__error => { fg: Color::LightRed }
        }
    };
}
