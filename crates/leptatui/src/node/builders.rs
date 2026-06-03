//! Convenience constructors for render-tree nodes.
//!
//! This module provides the public helper functions re-exported by
//! [`crate::node`] and [`crate::prelude`].

use std::rc::Rc;

use crate::component::Component;

use super::{
    component_node::ComponentNode,
    metadata::{NodeType, StyleMetadata},
    model::Node,
};

/// Creates a bordered block around a child node.
///
/// # Arguments
///
/// * `child` — Node-compatible value rendered inside the block.
///
/// # Returns
///
/// A [`Node::Block`] containing the provided child.
pub fn block(child: impl Into<Node>) -> Node {
    Node::Block {
        child: Box::new(child.into()),
        metadata: StyleMetadata::new(NodeType::Block),
    }
}

/// Creates a text node.
///
/// # Arguments
///
/// * `content` — Text content to render.
///
/// # Returns
///
/// A [`Node::Text`] containing the provided content.
pub fn text(content: impl Into<String>) -> Node {
    Node::Text {
        content: content.into(),
        metadata: StyleMetadata::new(NodeType::Text),
    }
}

/// Creates a horizontal row.
///
/// # Arguments
///
/// * `children` — Child nodes to divide across the row.
///
/// # Returns
///
/// A [`Node::Row`] containing the provided children.
pub fn row(children: impl IntoIterator<Item = Node>) -> Node {
    Node::Row {
        children: children.into_iter().collect(),
        metadata: StyleMetadata::new(NodeType::Row),
    }
}

/// Creates a vertical column.
///
/// # Arguments
///
/// * `children` — Child nodes to divide down the column.
///
/// # Returns
///
/// A [`Node::Column`] containing the provided children.
pub fn column(children: impl IntoIterator<Item = Node>) -> Node {
    Node::Column {
        children: children.into_iter().collect(),
        metadata: StyleMetadata::new(NodeType::Column),
    }
}

/// Creates a basic button.
///
/// # Arguments
///
/// * `label` — Button text to center inside a bordered area.
///
/// # Returns
///
/// A [`Node::Button`] containing the provided label.
pub fn button(label: impl Into<String>) -> Node {
    Node::Button {
        label: label.into(),
        metadata: StyleMetadata::new(NodeType::Button),
        on_press: None,
    }
}

/// Creates a dynamic child node.
///
/// # Arguments
///
/// * `child` — Closure that produces a node during render-tree traversal.
///
/// # Returns
///
/// A [`Node::Dynamic`] containing the provided child closure.
pub fn dynamic(child: impl Fn() -> Node + 'static) -> Node {
    Node::Dynamic(Rc::new(child))
}

/// Creates a component-boundary node.
///
/// # Arguments
///
/// * `component` — Component value to preserve as a render-tree boundary.
///
/// # Returns
///
/// A [`Node::Component`] containing the provided component.
pub fn component(component: impl Component + 'static) -> Node {
    Node::Component(ComponentNode::new(component))
}
