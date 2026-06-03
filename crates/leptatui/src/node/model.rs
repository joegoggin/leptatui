//! Render-tree node data structures.
//!
//! This module defines the node enum and its equality/debug behavior used by
//! node builders and renderers.

use std::{fmt, rc::Rc};

use super::{component_node::ComponentNode, dynamic::DynamicNode};

/// Minimal renderable node tree for hand-written terminal UI.
#[derive(Clone)]
pub enum Node {
    /// Bordered container around a child node.
    Block {
        /// Child node rendered inside the block's inner area.
        child: Box<Node>,
    },
    /// Plain text content.
    Text(String),
    /// Horizontally arranged children.
    Row(Vec<Node>),
    /// Vertically arranged children.
    Column(Vec<Node>),
    /// Basic bordered button label.
    Button(String),
    /// Child node produced when the tree is traversed.
    Dynamic(DynamicNode),
    /// Child component preserved as a tree boundary.
    Component(ComponentNode),
}

impl fmt::Debug for Node {
    /// Formats a node tree for diagnostics.
    ///
    /// Dynamic nodes avoid formatting their closures because closures do not
    /// implement [`fmt::Debug`].
    ///
    /// # Arguments
    ///
    /// * `f` — Formatter receiving the debug representation.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating whether formatting succeeded.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block { child } => f.debug_struct("Block").field("child", child).finish(),
            Self::Text(content) => f.debug_tuple("Text").field(content).finish(),
            Self::Row(children) => f.debug_tuple("Row").field(children).finish(),
            Self::Column(children) => f.debug_tuple("Column").field(children).finish(),
            Self::Button(label) => f.debug_tuple("Button").field(label).finish(),
            Self::Dynamic(_) => f.write_str("Dynamic(..)"),
            Self::Component(component) => f.debug_tuple("Component").field(component).finish(),
        }
    }
}

impl PartialEq for Node {
    /// Compares node trees by value, using pointer identity for deferred nodes.
    ///
    /// # Arguments
    ///
    /// * `other` — Node to compare with `self`.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the nodes are equal.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Block { child: left }, Self::Block { child: right }) => left == right,
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Row(left), Self::Row(right)) => left == right,
            (Self::Column(left), Self::Column(right)) => left == right,
            (Self::Button(left), Self::Button(right)) => left == right,
            (Self::Dynamic(left), Self::Dynamic(right)) => Rc::ptr_eq(left, right),
            (Self::Component(left), Self::Component(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

impl Eq for Node {}

impl From<String> for Node {
    /// Converts owned text into a text node.
    ///
    /// # Arguments
    ///
    /// * `value` — Text content to render.
    ///
    /// # Returns
    ///
    /// A [`Node::Text`] containing `value`.
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Node {
    /// Converts borrowed text into a text node.
    ///
    /// # Arguments
    ///
    /// * `value` — Text content to copy into the node.
    ///
    /// # Returns
    ///
    /// A [`Node::Text`] containing `value`.
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}
