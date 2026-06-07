//! Selector metadata attached to render-tree nodes.
//!
//! This module stores the type, id, class, inline-style, and focus metadata
//! used by stylesheet selectors during node rendering.

use crate::style::TuiStyle;

/// Static terminal element type used by style selectors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeType {
    /// Bordered container node.
    Block,
    /// Plain text node.
    Text,
    /// Horizontal layout node.
    Row,
    /// Vertical layout node.
    Column,
    /// Basic button node.
    Button,
}

/// Selector metadata stored with styleable render-tree nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleMetadata {
    node_type: NodeType,
    id: Option<String>,
    classes: Vec<String>,
    inline_style: Option<TuiStyle>,
    focused: bool,
}

impl StyleMetadata {
    /// Creates empty selector metadata for a node type.
    ///
    /// # Arguments
    ///
    /// * `node_type` — Static node type represented by the metadata.
    ///
    /// # Returns
    ///
    /// A [`StyleMetadata`] value with no id, classes, inline style, or focus.
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            id: None,
            classes: Vec::new(),
            inline_style: None,
            focused: false,
        }
    }

    /// Returns the style selector node type.
    ///
    /// # Returns
    ///
    /// A [`NodeType`] value used by type selectors.
    pub const fn node_type(&self) -> NodeType {
        self.node_type
    }

    /// Returns the optional id selector value.
    ///
    /// # Returns
    ///
    /// An [`Option<&str>`] containing the node id.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Returns class selector values in deterministic source order.
    ///
    /// # Returns
    ///
    /// A string slice containing class selector values.
    pub fn classes(&self) -> &[String] {
        &self.classes
    }

    /// Returns the inline style override, if present.
    ///
    /// # Returns
    ///
    /// An [`Option<TuiStyle>`] containing the inline style override.
    pub const fn inline_style(&self) -> Option<TuiStyle> {
        self.inline_style
    }

    /// Returns whether this node currently matches `:focus`.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether this node is focused.
    pub const fn is_focused(&self) -> bool {
        self.focused
    }

    /// Replaces the id selector value.
    ///
    /// # Arguments
    ///
    /// * `id` — Id selector value to store.
    pub fn set_id(&mut self, id: impl Into<String>) {
        self.id = Some(id.into());
    }

    /// Replaces class selector values by splitting an HTML-like class string.
    ///
    /// # Arguments
    ///
    /// * `classes` — Whitespace-separated class selector values.
    pub fn set_classes(&mut self, classes: impl Into<String>) {
        self.classes = classes
            .into()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
    }

    /// Replaces the inline style override.
    ///
    /// # Arguments
    ///
    /// * `style` — Inline style override to store.
    pub fn set_inline_style(&mut self, style: TuiStyle) {
        self.inline_style = Some(style);
    }

    /// Replaces the current focus pseudo-class state.
    ///
    /// # Arguments
    ///
    /// * `focused` — Whether this node should match `:focus`.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}
