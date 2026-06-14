//! Dynamic child storage for render-tree views.

use std::{cell::RefCell, rc::Rc};

use super::model::View;

/// Shared dynamic child that preserves compatible child state between refreshes.
#[derive(Clone)]
pub struct DynamicView {
    inner: Rc<DynamicViewInner>,
}

/// Deferred dynamic view state shared by cloned dynamic boundaries.
struct DynamicViewInner {
    child: Box<dyn Fn() -> View>,
    current: RefCell<Option<View>>,
}

impl DynamicView {
    /// Creates a dynamic view boundary from a child-producing closure.
    pub(crate) fn new(child: impl Fn() -> View + 'static) -> Self {
        Self {
            inner: Rc::new(DynamicViewInner {
                child: Box::new(child),
                current: RefCell::new(None),
            }),
        }
    }

    /// Returns whether two dynamic boundaries share the same storage.
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    /// Refreshes the current child and reads it for the duration of `read`.
    pub(crate) fn with_view<R>(&self, read: impl FnOnce(&View) -> R) -> R {
        self.refresh();

        let current = self.inner.current.borrow();
        read(current.as_ref().expect("dynamic view should be refreshed"))
    }

    /// Refreshes the current child and mutates it for the duration of `write`.
    pub(crate) fn with_view_mut<R>(&self, write: impl FnOnce(&mut View) -> R) -> R {
        self.refresh();

        let mut current = self.inner.current.borrow_mut();
        write(current.as_mut().expect("dynamic view should be refreshed"))
    }

    /// Rebuilds the child view and reconciles compatible state from the previous child.
    fn refresh(&self) {
        let mut next = (self.inner.child)();
        let mut current = self.inner.current.borrow_mut();

        if let Some(previous) = current.as_ref() {
            crate::__private::__reconcile_view(&mut next, previous);
        }

        *current = Some(next);
    }
}
