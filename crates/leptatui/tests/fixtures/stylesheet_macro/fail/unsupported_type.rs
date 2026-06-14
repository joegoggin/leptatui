//! Fail fixture for unsupported `stylesheet!` type selectors.

use leptatui::prelude::*;

/// Defines a stylesheet with an unsupported view type selector.
fn main() {
    let _ = stylesheet! {
        Panel => { fg: Color::White }
    };
}
