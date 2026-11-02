//! Fail fixture for a BEM suffix beneath a type selector.

use leptatui::prelude::*;

/// Defines a BEM parent suffix whose parent is not a class selector.
fn main() {
    let _styles = stylesheet! {
        Button => {
            &--primary => { fg: Color::White }
        }
    };
}
