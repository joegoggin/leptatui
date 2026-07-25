//! Fail fixture for an incompatible typed layout declaration value.

use leptatui::prelude::*;

/// Assigns overflow behavior to the display property.
fn main() {
    let _styles = stylesheet! {
        Text => { display: Overflow::Hidden }
    };
}
