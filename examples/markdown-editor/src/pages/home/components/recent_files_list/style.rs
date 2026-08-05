//! Recent-files list stylesheet registration.

use leptatui::prelude::*;

/// Registers the recent-files list stylesheet with the active component.
pub(super) fn use_recent_files_list_styles() {
    stylesheet! {
        .recent-files => {
            &__title => {
                fg: Color::White,
                modifier: Modifier::BOLD
            }
            &__empty => { fg: Color::DarkGray }
            &__error => { fg: Color::LightRed }
        }
    };
}
