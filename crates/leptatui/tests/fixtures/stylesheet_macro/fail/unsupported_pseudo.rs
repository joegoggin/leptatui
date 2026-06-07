//! Fail fixture for unsupported `stylesheet!` pseudo-selectors.

use leptatui::prelude::*;

/// Defines a stylesheet with an unsupported pseudo-selector.
fn main() {
    let _ = stylesheet! {
        Button:hover => { bg: Color::Blue }
    };
}
