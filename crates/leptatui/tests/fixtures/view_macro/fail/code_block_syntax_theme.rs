//! Fail fixture for removed `CodeBlock` syntax-theme selection.

use leptatui::prelude::*;

/// Triggers the `CodeBlock` unsupported-attribute validation failure.
fn main() {
    let _ = view! {
        <CodeBlock syntax_theme={Color::Blue}>"fn main() {}"</CodeBlock>
    };
}
