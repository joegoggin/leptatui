//! Dynamic child storage for render-tree views.

use std::rc::Rc;

use super::model::View;

/// Shared dynamic child that can produce a fresh view during traversal.
pub type DynamicView = Rc<dyn Fn() -> View>;
