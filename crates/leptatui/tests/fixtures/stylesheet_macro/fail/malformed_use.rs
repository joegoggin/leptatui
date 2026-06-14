//! Fail fixture for malformed `stylesheet!` imports.

use leptatui::prelude::*;

/// Defines a placeholder style module.
fn colors() -> StyleModule {
    StyleModule::new()
}

/// Attempts to import a module with an incomplete alias clause.
fn main() {
    let _styles = stylesheet! {
        @use colors as;

        Text => { fg: Color::White }
    };
}
