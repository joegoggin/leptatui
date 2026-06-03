//! Pass fixture for basic `view!` expansion.
//!
//! This binary verifies supported element attributes populate selector metadata
//! while nested elements lower into the expected node tree.

use leptatui::prelude::*;

/// Exercises basic element expansion and accepted attributes.
fn main() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let node: Node = view! {
        <Block class="card active" id="main" style={style}>
            <Text class={"label"}>"hello"</Text>
        </Block>
    };

    match node {
        Node::Block { child, metadata } => {
            assert_eq!(metadata.node_type(), NodeType::Block);
            assert_eq!(metadata.id(), Some("main"));
            assert_eq!(
                metadata.classes(),
                &[String::from("card"), String::from("active")]
            );
            assert_eq!(metadata.inline_style(), Some(style));

            match *child {
                Node::Text { content, metadata } => {
                    assert_eq!(content, "hello");
                    assert_eq!(metadata.node_type(), NodeType::Text);
                    assert_eq!(metadata.classes(), &[String::from("label")]);
                }
                other => panic!("expected text child, got {other:?}"),
            }
        }
        other => panic!("expected block node, got {other:?}"),
    }
}
