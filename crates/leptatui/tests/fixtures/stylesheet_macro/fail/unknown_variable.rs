//! Fail fixture for unknown `stylesheet!` variables.

use leptatui::prelude::*;
/// Defines a stylesheet that references an unknown variable.
fn main() {
    let _styles = stylesheet! {
        $primary: Color::LightCyan;

        Text => { fg: $missing }
    };
}
