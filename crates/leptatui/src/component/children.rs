//! Component children callback aliases.
//!
//! These aliases model nested `view!` component content as deferred view lists.

use crate::view::View;

/// Deferred children that can be rendered once by a component.
pub type Children = Box<dyn FnOnce() -> Vec<View>>;

/// Deferred children that can be rendered repeatedly by a component.
pub type ChildrenFn = Box<dyn Fn() -> Vec<View>>;

/// Deferred children that can be rendered mutably by a component.
pub type ChildrenMut = Box<dyn FnMut() -> Vec<View>>;
