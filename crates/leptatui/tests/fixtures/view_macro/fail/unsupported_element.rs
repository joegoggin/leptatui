//! Fail fixture for unsupported `view!` attributes.
//!
//! This binary triggers the diagnostic for attributes outside the accepted
//! `class`, `id`, and `style` set.

use leptatui::prelude::*;

/// Triggers the unsupported-attribute validation failure.
fn main() {
    let _ = view! {
        <Block data_id="bad">
            <Text>"bad"</Text>
        </Block>
    };
}
