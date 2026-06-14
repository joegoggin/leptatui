//! Fail fixture for unknown `stylesheet!` mixins.

use leptatui::prelude::*;

/// Triggers unknown mixin include diagnostics.
fn main() {
    let _styles = stylesheet! {
        Text => { @include missing }
    };
}
