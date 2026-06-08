//! Fail fixture for top-level nested parent references.

use leptatui::prelude::*;

/// Defines a stylesheet with a parent reference and no parent selector.
fn main() {
    let _styles = stylesheet! {
        &:focus => { fg: Color::White }
    };
}
