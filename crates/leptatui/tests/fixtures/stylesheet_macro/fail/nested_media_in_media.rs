//! Fail fixture for one nested media query inside another.

use leptatui::prelude::*;

/// Defines nested media conditions that should use one `and` query instead.
fn main() {
    let _styles = stylesheet! {
        .panel => {
            @media (max-width: 80) {
                @media (min-height: 24) {
                    padding: TuiSpacing::ZERO
                }
            }
        }
    };
}
