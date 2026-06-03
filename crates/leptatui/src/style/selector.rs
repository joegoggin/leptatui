use crate::node::{NodeType, StyleMetadata};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleSelector {
    Type(NodeType),
    Class(String),
    Id(String),
}

impl StyleSelector {
    pub const fn node_type(node_type: NodeType) -> Self {
        Self::Type(node_type)
    }

    pub fn class(class: impl Into<String>) -> Self {
        Self::Class(class.into())
    }

    pub fn id(id: impl Into<String>) -> Self {
        Self::Id(id.into())
    }

    pub(crate) fn matches(&self, metadata: &StyleMetadata) -> bool {
        match self {
            Self::Type(node_type) => metadata.node_type() == *node_type,
            Self::Class(class) => metadata.classes().iter().any(|value| value == class),
            Self::Id(id) => metadata.id() == Some(id.as_str()),
        }
    }

    pub(crate) const fn specificity(&self) -> Specificity {
        match self {
            Self::Type(_) => Specificity::Type,
            Self::Class(_) => Specificity::Class,
            Self::Id(_) => Specificity::Id,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Specificity {
    Type,
    Class,
    Id,
}
