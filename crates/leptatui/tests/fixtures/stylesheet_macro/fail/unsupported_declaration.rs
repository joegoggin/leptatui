//! Fail fixture for unsupported `stylesheet!` declarations.

use leptatui::prelude::*;

/// Defines a stylesheet with an unsupported declaration name.
fn main() {
    let _ = stylesheet! {
        Text => { color: Color::White }
    };
}
