//! Dynamic child storage for render-tree nodes.

use std::rc::Rc;

use super::model::Node;

/// Shared dynamic child that can produce a fresh node during traversal.
pub type DynamicNode = Rc<dyn Fn() -> Node>;
