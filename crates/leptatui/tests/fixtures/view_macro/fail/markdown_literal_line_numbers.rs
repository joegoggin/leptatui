//! Fail fixture for a string-valued Markdown boolean option.

use leptatui::prelude::*;

/// Triggers typed Markdown option validation.
fn main() {
    let _ = view! {
        <Markdown src="guide.md" line_numbers="true" />
    };
}
