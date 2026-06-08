//! Fail fixture for duplicate `stylesheet!` mixins.

use leptatui::prelude::*;

/// Triggers duplicate mixin diagnostics.
fn main() {
    let _styles = stylesheet! {
        @mixin chrome {
            bg: Color::Black
        }

        @mixin chrome {
            fg: Color::White
        }

        Text => { @include chrome }
    };
}
