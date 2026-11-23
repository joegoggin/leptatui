//! Rejects obsolete or unterminated stylesheet syntax.

use leptatui::prelude::*;

/// Exercises a stylesheet syntax error.
fn main() {
    let _ = stylesheet! {
        .panel { fg: Color::White Button { fg: Color::Yellow; } }
    };
}
