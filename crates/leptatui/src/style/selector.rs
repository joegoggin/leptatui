//! Selectors for matching node style metadata.
//!
//! This module defines the selectors used by [`Stylesheet`](super::Stylesheet)
//! resolution and maps selectors to cascade specificity groups.

use crate::node::{NodeType, StyleMetadata};

/// Selector used to match a style rule against a node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleSelector {
    /// Matches nodes by their [`NodeType`].
    Type(NodeType),
    /// Matches nodes containing the provided class name.
    Class(String),
    /// Matches nodes with the provided id.
    Id(String),
    /// Matches nodes currently marked as focused.
    Focus,
    /// Matches nodes that satisfy every nested selector.
    Compound(Vec<StyleSelector>),
}

impl StyleSelector {
    /// Creates a selector that matches a node type.
    ///
    /// # Arguments
    ///
    /// * `node_type` — Node type to match.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Type`] selector.
    pub const fn node_type(node_type: NodeType) -> Self {
        Self::Type(node_type)
    }

    /// Creates a selector that matches a class name.
    ///
    /// # Arguments
    ///
    /// * `class` — Class name to match.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Class`] selector.
    pub fn class(class: impl Into<String>) -> Self {
        Self::Class(class.into())
    }

    /// Creates a selector that matches an id.
    ///
    /// # Arguments
    ///
    /// * `id` — Id value to match.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Id`] selector.
    pub fn id(id: impl Into<String>) -> Self {
        Self::Id(id.into())
    }

    /// Creates a selector that matches focused nodes.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Focus`] selector.
    pub const fn focus() -> Self {
        Self::Focus
    }

    /// Creates a selector that matches only when every nested selector matches.
    ///
    /// # Arguments
    ///
    /// * `selectors` — Selector list to match together.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Compound`] selector.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `selectors` is empty.
    pub fn compound(selectors: impl Into<Vec<Self>>) -> Self {
        let selectors = selectors.into();
        debug_assert!(
            !selectors.is_empty(),
            "compound selector requires at least one nested selector",
        );

        Self::Compound(selectors)
    }

    /// Returns whether this selector matches node style metadata.
    ///
    /// # Arguments
    ///
    /// * `metadata` — Node selector metadata to inspect.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the selector matches.
    pub(crate) fn matches(&self, metadata: &StyleMetadata) -> bool {
        match self {
            Self::Type(node_type) => metadata.node_type() == *node_type,
            Self::Class(class) => metadata.classes().iter().any(|value| value == class),
            Self::Id(id) => metadata.id() == Some(id.as_str()),
            Self::Focus => metadata.is_focused(),
            Self::Compound(selectors) => {
                selectors.iter().all(|selector| selector.matches(metadata))
            }
        }
    }

    /// Returns the cascade specificity for this selector.
    ///
    /// # Returns
    ///
    /// A [`Specificity`] value used to order rule application.
    pub(crate) fn specificity(&self) -> Specificity {
        match self {
            Self::Type(_) => Specificity::Type,
            Self::Class(_) | Self::Focus => Specificity::Class,
            Self::Id(_) => Specificity::Id,
            Self::Compound(selectors) => selectors
                .iter()
                .map(Self::specificity)
                .max()
                .unwrap_or(Specificity::Type),
        }
    }
}

/// Cascade specificity group for stylesheet rule application.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Specificity {
    /// Type selector specificity.
    Type,
    /// Class and focus selector specificity.
    Class,
    /// Id selector specificity.
    Id,
}
