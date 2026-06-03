//! Pass fixture for basic `view!` expansion.
//!
//! This binary verifies supported element attributes are accepted and ignored
//! while nested elements lower into the expected node tree.

use leptatui::prelude::*;

/// Exercises basic element expansion and accepted attributes.
fn main() {
    let node: Node = view! {
        <Block class="card" id="main" style={leptatui::TuiStyle::new()}>
            <Text>"hello"</Text>
        </Block>
    };

    assert_eq!(node, block(text("hello")));
}
