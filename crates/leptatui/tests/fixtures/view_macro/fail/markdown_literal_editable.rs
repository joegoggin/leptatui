//! Fail fixture for a string-valued Markdown editable option.

use leptatui::prelude::*;

/// Triggers typed Markdown editable-option validation.
fn main() {
    let _ = view! {
        <Markdown src="guide.md" editable="true" />
    };
}
