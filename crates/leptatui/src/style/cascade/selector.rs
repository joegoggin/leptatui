//! Selectors for matching view style metadata.
//!
//! This module defines the selectors used by [`Stylesheet`](super::Stylesheet)
//! resolution, including descendant selector paths, and maps selectors to CSS
//! cascade specificity.

use crate::view::{StyleMetadata, ViewType};

/// Selector used to match a style rule against a view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleSelector {
    /// Matches views by their [`ViewType`].
    Type(ViewType),
    /// Matches views containing the provided class name.
    Class(String),
    /// Matches views with the provided id.
    Id(String),
    /// Matches views currently marked as focused.
    Focus,
    /// Matches views currently marked as active.
    Active,
    /// Matches views currently marked as being in insert mode.
    Insert,
    /// Matches views currently marked as being in a visual mode.
    Visual,
    /// Matches views currently marked as visited.
    Visited,
    /// Matches views that satisfy every nested selector.
    Compound(Vec<StyleSelector>),
    /// Matches a target selector when its ordered ancestor chain matches.
    Descendant {
        /// Ancestor selectors ordered from outermost to innermost.
        ancestors: Vec<StyleSelector>,
        /// Selector matched against the current view.
        target: Box<StyleSelector>,
    },
}

impl StyleSelector {
    /// Creates a selector that matches a view type.
    ///
    /// # Arguments
    ///
    /// * `view_type` — View type to match.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Type`] selector.
    pub const fn view_type(view_type: ViewType) -> Self {
        Self::Type(view_type)
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

    /// Creates a selector that matches focused views.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Focus`] selector.
    pub const fn focus() -> Self {
        Self::Focus
    }

    /// Creates a selector that matches active views.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Active`] selector.
    pub const fn active() -> Self {
        Self::Active
    }

    /// Creates a selector that matches views in insert mode.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Insert`] selector.
    pub const fn insert() -> Self {
        Self::Insert
    }

    /// Creates a selector that matches views in a visual mode.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Visual`] selector.
    pub const fn visual() -> Self {
        Self::Visual
    }

    /// Creates a selector that matches visited views.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Visited`] selector.
    pub const fn visited() -> Self {
        Self::Visited
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

    /// Creates a selector that matches a target below matching ancestors.
    ///
    /// # Arguments
    ///
    /// * `ancestors` — Selector chain ordered from outermost ancestor to
    ///   innermost ancestor.
    /// * `target` — Selector matched against the current view.
    ///
    /// # Returns
    ///
    /// A [`StyleSelector::Descendant`] selector.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `ancestors` is empty.
    pub fn descendant(ancestors: impl Into<Vec<Self>>, target: impl Into<Self>) -> Self {
        let ancestors = ancestors.into();
        debug_assert!(
            !ancestors.is_empty(),
            "descendant selector requires at least one ancestor selector",
        );

        Self::Descendant {
            ancestors,
            target: Box::new(target.into()),
        }
    }

    /// Returns whether this selector matches view style metadata and ancestors.
    ///
    /// # Arguments
    ///
    /// * `metadata` — View selector metadata to inspect.
    /// * `ancestors` — Ancestor metadata ordered from outermost to innermost.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the selector matches.
    pub(crate) fn matches(&self, metadata: &StyleMetadata, ancestors: &[StyleMetadata]) -> bool {
        match self {
            Self::Type(view_type) => metadata.view_type() == *view_type,
            Self::Class(class) => metadata.classes().iter().any(|value| value == class),
            Self::Id(id) => metadata.id() == Some(id.as_str()),
            Self::Focus => metadata.is_focused(),
            Self::Active => metadata.is_active(),
            Self::Insert => metadata.is_insert(),
            Self::Visual => metadata.is_visual(),
            Self::Visited => metadata.is_visited(),
            Self::Compound(selectors) => selectors
                .iter()
                .all(|selector| selector.matches(metadata, ancestors)),
            Self::Descendant {
                ancestors: required_ancestors,
                target,
            } => {
                target.matches(metadata, ancestors)
                    && matches_ancestor_chain(required_ancestors, ancestors)
            }
        }
    }

    /// Returns this selector's CSS specificity tuple.
    ///
    /// The tuple is ordered as `(ids, classes_and_pseudos, types)`.
    ///
    /// # Returns
    ///
    /// A tuple containing this selector's CSS-style specificity.
    pub fn css_specificity(&self) -> (u16, u16, u16) {
        self.specificity().as_tuple()
    }

    /// Returns the cascade specificity for this selector.
    ///
    /// # Returns
    ///
    /// A [`Specificity`] value used to order rule application.
    pub(crate) fn specificity(&self) -> Specificity {
        match self {
            Self::Type(_) => Specificity::TYPE,
            Self::Class(_)
            | Self::Focus
            | Self::Active
            | Self::Insert
            | Self::Visual
            | Self::Visited => Specificity::CLASS,
            Self::Id(_) => Specificity::ID,
            Self::Compound(selectors) => selectors
                .iter()
                .fold(Specificity::ZERO, |specificity, selector| {
                    specificity + selector.specificity()
                }),
            Self::Descendant { ancestors, target } => ancestors
                .iter()
                .fold(target.specificity(), |specificity, selector| {
                    specificity + selector.specificity()
                }),
        }
    }
}

/// Returns whether the available ancestors contain the required selector chain.
///
/// # Arguments
///
/// * `required_ancestors` — Selector chain required by a descendant selector.
/// * `ancestors` — Available ancestor metadata ordered from outermost to
///   innermost.
///
/// # Returns
///
/// A [`bool`] indicating whether each required ancestor selector matches in
/// order.
fn matches_ancestor_chain(
    required_ancestors: &[StyleSelector],
    ancestors: &[StyleMetadata],
) -> bool {
    let mut required = required_ancestors.iter();
    let Some(mut current) = required.next() else {
        return true;
    };

    for ancestor in ancestors {
        if current.matches(ancestor, &[]) {
            let Some(next) = required.next() else {
                return true;
            };
            current = next;
        }
    }

    false
}

/// CSS cascade specificity for stylesheet rule application.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Specificity {
    /// Number of id selectors in the selector.
    ids: u16,
    /// Number of class and pseudo selectors in the selector.
    classes_and_pseudos: u16,
    /// Number of type selectors in the selector.
    types: u16,
}

impl Specificity {
    /// Specificity for an empty selector.
    pub(crate) const ZERO: Self = Self {
        ids: 0,
        classes_and_pseudos: 0,
        types: 0,
    };
    /// Specificity contribution from an id selector.
    const ID: Self = Self {
        ids: 1,
        classes_and_pseudos: 0,
        types: 0,
    };
    /// Specificity contribution from a class or pseudo selector.
    const CLASS: Self = Self {
        ids: 0,
        classes_and_pseudos: 1,
        types: 0,
    };
    /// Specificity contribution from a type selector.
    const TYPE: Self = Self {
        ids: 0,
        classes_and_pseudos: 0,
        types: 1,
    };

    /// Returns the specificity components in cascade comparison order.
    ///
    /// # Returns
    ///
    /// A [`tuple`](prim@tuple) containing id, class-or-pseudo, and type counts.
    const fn as_tuple(self) -> (u16, u16, u16) {
        (self.ids, self.classes_and_pseudos, self.types)
    }
}

impl ::core::ops::Add for Specificity {
    /// Specificity sum output type.
    type Output = Self;

    /// Adds specificity components with saturating arithmetic.
    ///
    /// # Arguments
    ///
    /// * `rhs` — Specificity value to add to this value.
    ///
    /// # Returns
    ///
    /// A [`Specificity`] containing the combined selector weight.
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            ids: self.ids.saturating_add(rhs.ids),
            classes_and_pseudos: self
                .classes_and_pseudos
                .saturating_add(rhs.classes_and_pseudos),
            types: self.types.saturating_add(rhs.types),
        }
    }
}
