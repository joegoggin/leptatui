//! Fail fixture for a Markdown source attribute on another element.

use leptatui::prelude::*;

/// Triggers the misplaced Markdown source validation failure.
fn main() {
    let _ = view! {
        <Paragraph source="# Guide">"bad"</Paragraph>
    };
}
