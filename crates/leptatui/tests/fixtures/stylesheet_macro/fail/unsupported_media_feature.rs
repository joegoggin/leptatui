//! Fail fixture for unsupported `stylesheet!` media query features.

use leptatui::prelude::*;

/// Defines a stylesheet with an unsupported media feature.
fn main() {
    let _styles = stylesheet! {
        @media (min-depth: 80) {
            Text => { fg: Color::White }
        }
    };
}
