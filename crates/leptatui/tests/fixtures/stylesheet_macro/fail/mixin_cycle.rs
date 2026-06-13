//! Fail fixture for cyclic `stylesheet!` mixin includes.

use leptatui::prelude::*;

/// Defines mixins that include each other.
fn main() {
    let _module = stylesheet! {
        @mixin first {
            @include second
        }

        @mixin second {
            @include first
        }
    };
}
