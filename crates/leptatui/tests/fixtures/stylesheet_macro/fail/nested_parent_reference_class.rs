//! Fail fixture for unsupported nested parent reference selectors.

use leptatui::prelude::*;

/// Defines a stylesheet with an unsupported nested parent-reference selector.
fn main() {
    let _styles = stylesheet! {
        Text => {
            &.primary => { fg: Color::White }
        }
    };
}
