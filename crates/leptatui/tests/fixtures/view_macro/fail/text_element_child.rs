//! Fail fixture for nested elements inside `Text`.
//!
//! This binary triggers the diagnostic for text-like elements that receive
//! element children instead of text content.

use leptatui::prelude::*;

/// Triggers the text-child validation failure.
fn main() {
    let _ = view! {
        <Text>
            <Block>
                <Text>"bad"</Text>
            </Block>
        </Text>
    };
}
