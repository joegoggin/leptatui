//! Fail fixture for a BEM parent suffix without a parent selector.

use leptatui::prelude::*;

/// Defines a top-level BEM parent-reference selector.
fn main() {
    let _styles = stylesheet! {
        &__button => { fg: Color::White }
    };
}
