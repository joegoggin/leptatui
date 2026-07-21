//! Pass fixture for open `stylesheet!` type selectors.

use leptatui::prelude::*;

/// Defines a stylesheet for an application-owned view type name.
fn main() {
    let stylesheet: Stylesheet = stylesheet! {
        Panel => { fg: Color::White }
    };

    let _ = stylesheet;
    assert_eq!(ViewType::new("Panel").name(), "Panel");
}
