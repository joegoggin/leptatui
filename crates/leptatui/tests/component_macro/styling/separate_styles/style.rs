//! Fixture stylesheet registration from a sibling Rust module.

use leptatui::{Color, stylesheet};

/// Registers root and BEM content styles with the active fixture component.
pub(super) fn use_separate_style_fixture_styles() {
    stylesheet! {
        .separate-style => {
            fg: Color::Yellow

            &__content => { bg: Color::Blue }
        }
    };
}
