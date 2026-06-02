use super::storage;

/// Guard that pops a Leptatui context frame when a render scope ends.
pub(super) struct ContextScopeGuard;

impl ContextScopeGuard {
    /// Pushes a new empty context frame for the current thread.
    ///
    /// # Returns
    ///
    /// A [`ContextScopeGuard`] that restores the previous context stack on
    /// drop.
    pub(super) fn enter() -> Self {
        storage::push_frame();
        Self
    }
}

impl Drop for ContextScopeGuard {
    /// Pops the current context frame when the scope guard is dropped.
    fn drop(&mut self) {
        debug_assert!(storage::pop_frame(), "context scope stack underflow");
    }
}
