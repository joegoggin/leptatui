//! Fail fixture for removed `Markdown` syntax-theme selection.

use leptatui::prelude::*;

/// Triggers the `Markdown` unsupported-attribute validation failure.
fn main() {
    let _ = view! {
        <Markdown src="guide.md" syntax_theme={Color::Blue} />
    };
}
