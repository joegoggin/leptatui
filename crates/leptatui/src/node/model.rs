//! Render-tree node data structures.
//!
//! This module defines the node enum and its equality/debug behavior used by
//! node builders and renderers.

use std::{fmt, rc::Rc};

use super::{
    component_node::ComponentNode,
    dynamic::DynamicNode,
    metadata::{NodeType, StyleMetadata},
};

/// Minimal renderable node tree for hand-written terminal UI.
#[derive(Clone)]
pub enum Node {
    /// Bordered container around a child node.
    Block {
        /// Child node rendered inside the block's inner area.
        child: Box<Node>,
        /// Selector metadata for matching this node.
        metadata: StyleMetadata,
    },
    /// Plain text content.
    Text {
        /// Text content to render.
        content: String,
        /// Selector metadata for matching this node.
        metadata: StyleMetadata,
    },
    /// Horizontally arranged children.
    Row {
        /// Child nodes divided across the row.
        children: Vec<Node>,
        /// Selector metadata for matching this node.
        metadata: StyleMetadata,
    },
    /// Vertically arranged children.
    Column {
        /// Child nodes divided down the column.
        children: Vec<Node>,
        /// Selector metadata for matching this node.
        metadata: StyleMetadata,
    },
    /// Basic bordered button label.
    Button {
        /// Button label to render.
        label: String,
        /// Selector metadata for matching this node.
        metadata: StyleMetadata,
    },
    /// Child node produced when the tree is traversed.
    Dynamic(DynamicNode),
    /// Child component preserved as a tree boundary.
    Component(ComponentNode),
}

impl Node {
    /// Returns selector metadata for styleable static nodes.
    pub fn style_metadata(&self) -> Option<&StyleMetadata> {
        match self {
            Self::Block { metadata, .. }
            | Self::Text { metadata, .. }
            | Self::Row { metadata, .. }
            | Self::Column { metadata, .. }
            | Self::Button { metadata, .. } => Some(metadata),
            Self::Dynamic(_) | Self::Component(_) => None,
        }
    }

    /// Returns mutable selector metadata for styleable static nodes.
    pub fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        match self {
            Self::Block { metadata, .. }
            | Self::Text { metadata, .. }
            | Self::Row { metadata, .. }
            | Self::Column { metadata, .. }
            | Self::Button { metadata, .. } => Some(metadata),
            Self::Dynamic(_) | Self::Component(_) => None,
        }
    }

    /// Sets an id selector value on a styleable node.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_id(id);
        }

        self
    }

    /// Sets class selector values on a styleable node.
    pub fn with_classes(mut self, classes: impl Into<String>) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_classes(classes);
        }

        self
    }

    /// Sets an inline style override on a styleable node.
    pub fn with_inline_style(mut self, style: crate::style::TuiStyle) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_inline_style(style);
        }

        self
    }

    /// Sets the current focus pseudo-class state on a styleable node.
    pub fn with_focus(mut self, focused: bool) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_focused(focused);
        }

        self
    }
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
            Self::Block { child, metadata } => f
                .debug_struct("Block")
                .field("child", child)
                .field("metadata", metadata)
                .finish(),
            Self::Text { content, metadata } => f
                .debug_struct("Text")
                .field("content", content)
                .field("metadata", metadata)
                .finish(),
            Self::Row { children, metadata } => f
                .debug_struct("Row")
                .field("children", children)
                .field("metadata", metadata)
                .finish(),
            Self::Column { children, metadata } => f
                .debug_struct("Column")
                .field("children", children)
                .field("metadata", metadata)
                .finish(),
            Self::Button { label, metadata } => f
                .debug_struct("Button")
                .field("label", label)
                .field("metadata", metadata)
                .finish(),
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
            (
                Self::Block {
                    child: left_child,
                    metadata: left_metadata,
                },
                Self::Block {
                    child: right_child,
                    metadata: right_metadata,
                },
            ) => left_child == right_child && left_metadata == right_metadata,
            (
                Self::Text {
                    content: left_content,
                    metadata: left_metadata,
                },
                Self::Text {
                    content: right_content,
                    metadata: right_metadata,
                },
            ) => left_content == right_content && left_metadata == right_metadata,
            (
                Self::Row {
                    children: left_children,
                    metadata: left_metadata,
                },
                Self::Row {
                    children: right_children,
                    metadata: right_metadata,
                },
            ) => left_children == right_children && left_metadata == right_metadata,
            (
                Self::Column {
                    children: left_children,
                    metadata: left_metadata,
                },
                Self::Column {
                    children: right_children,
                    metadata: right_metadata,
                },
            ) => left_children == right_children && left_metadata == right_metadata,
            (
                Self::Button {
                    label: left_label,
                    metadata: left_metadata,
                },
                Self::Button {
                    label: right_label,
                    metadata: right_metadata,
                },
            ) => left_label == right_label && left_metadata == right_metadata,
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
        Self::Text {
            content: value,
            metadata: StyleMetadata::new(NodeType::Text),
        }
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
        Self::Text {
            content: value.to_owned(),
            metadata: StyleMetadata::new(NodeType::Text),
        }
    }
}
