//! Component-boundary storage for render-tree views.
//!
//! This module wraps component values so they can live inside a cloneable view
//! tree while preserving component state and render-scope context between
//! events.

use std::{cell::RefCell, fmt, rc::Rc};

use crossterm::event::{Event, KeyEvent};

use crate::{
    app::{AppControl, Result},
    component::{Component, KeyControl, RenderCtx},
    context::ContextScope,
};

/// Shared component boundary stored inside a render tree.
#[derive(Clone)]
pub struct ComponentView {
    /// Shared mutable component stored behind the view boundary.
    inner: Rc<RefCell<dyn Component>>,
    /// Persistent context scope owned by this component subtree.
    context: ContextScope,
}

impl ComponentView {
    /// Creates a component boundary from a component value.
    ///
    /// # Arguments
    ///
    /// * `component` — Component value stored behind this render-tree boundary.
    ///
    /// # Returns
    ///
    /// A [`ComponentView`] containing the provided component.
    pub(crate) fn new(component: impl Component + 'static) -> Self {
        Self {
            inner: Rc::new(RefCell::new(component)),
            context: ContextScope::new(),
        }
    }

    /// Renders the stored component inside its reset context scope.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the view boundary.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if the component render path performs
    /// terminal I/O that fails.
    pub(crate) fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        self.context
            .with_reset(|| self.inner.borrow_mut().render(ctx))
    }

    /// Handles an event inside this component's existing context scope.
    ///
    /// # Arguments
    ///
    /// * `event` — Event dispatched through this component boundary.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value returned by the component.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if the component event path performs
    /// terminal I/O that fails.
    pub(crate) fn handle_event(&self, event: Event) -> Result<AppControl> {
        self.context
            .with(|| self.inner.borrow_mut().handle_event(event))
    }

    /// Dispatches a key event through custom handlers only.
    #[doc(hidden)]
    pub(crate) fn dispatch_key_event(&self, key: KeyEvent) -> Result<KeyControl> {
        self.context
            .with(|| self.inner.borrow_mut().__dispatch_key_event(key))
    }

    /// Returns the number of focusable controls inside the component boundary.
    #[doc(hidden)]
    pub(crate) fn focusable_count(&self) -> usize {
        self.context
            .with(|| self.inner.borrow().__focusable_count())
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    pub(crate) fn focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.context
            .with(|| self.inner.borrow().__focused_index_inner(index))
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    pub(crate) fn set_focus_by_index_inner(&self, target: usize, index: &mut usize) {
        self.context.with(|| {
            self.inner
                .borrow_mut()
                .__set_focus_by_index_inner(target, index)
        });
    }

    /// Activates the focused control inside the component boundary, if any.
    #[doc(hidden)]
    pub(crate) fn activate_focused_button(&self) -> Option<AppControl> {
        self.context
            .with(|| self.inner.borrow().__activate_focused_button())
    }

    /// Compares two component boundaries by shared storage identity.
    ///
    /// # Arguments
    ///
    /// * `other` — Component boundary to compare with `self`.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether both boundaries point to the same
    /// component storage.
    pub(super) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for ComponentView {
    /// Formats the component boundary without borrowing the stored component.
    ///
    /// # Arguments
    ///
    /// * `f` — Formatter receiving the debug representation.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating whether formatting succeeded.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentView").finish()
    }
}
