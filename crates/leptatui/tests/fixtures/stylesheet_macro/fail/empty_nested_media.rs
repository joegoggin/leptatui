//! Fail fixture for an empty media query nested inside a stylesheet rule.

use leptatui::prelude::*;

/// Defines a nested media block without declarations or rules.
fn main() {
    let _styles = stylesheet! {
        .panel => {
            @media (max-width: 80) {}
        }
    };
}
