//! Context-scope guard management.
//!
//! This module owns the RAII guard that pushes a context frame on entry and
//! restores the previous stack state on drop.

use super::storage;

/// Owned context frame for one component subtree.
#[derive(Clone)]
pub(crate) struct ContextScope {
    frame: storage::ContextFrame,
}

impl ContextScope {
    /// Creates an empty component context scope.
    ///
    /// # Returns
    ///
    /// A [`ContextScope`] with a reusable empty context frame.
    pub(crate) fn new() -> Self {
        Self {
            frame: storage::new_frame(),
        }
    }

    /// Runs a closure with this scope added to the active context stack.
    ///
    /// # Arguments
    ///
    /// * `render` — Closure that can provide and read scoped context values.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub(crate) fn with<R>(&self, render: impl FnOnce() -> R) -> R {
        let _scope = ContextScopeGuard::enter(&self.frame);
        render()
    }

    /// Clears this scope, then runs a closure with it active.
    ///
    /// # Arguments
    ///
    /// * `render` — Closure that can repopulate the cleared context frame.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `render`.
    pub(crate) fn with_reset<R>(&self, render: impl FnOnce() -> R) -> R {
        storage::clear_frame(&self.frame);
        self.with(render)
    }
}

/// Guard that pops a Leptatui context frame when a render scope ends.
pub(super) struct ContextScopeGuard;

impl ContextScopeGuard {
    /// Pushes an existing context frame for the current thread.
    ///
    /// # Returns
    ///
    /// A [`ContextScopeGuard`] that restores the previous context stack on
    /// drop.
    pub(super) fn enter(frame: &storage::ContextFrame) -> Self {
        storage::push_frame(frame);
        Self
    }
}

impl Drop for ContextScopeGuard {
    /// Pops the current context frame when the scope guard is dropped.
    fn drop(&mut self) {
        debug_assert!(storage::pop_frame(), "context scope stack underflow");
    }
}
