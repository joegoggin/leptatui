//! Rejects obsolete or unterminated stylesheet syntax.

use leptatui::prelude::*;

/// Exercises a stylesheet syntax error.
fn main() {
    let _ = stylesheet! {
        Text { fg: Color::White }
    };
}
