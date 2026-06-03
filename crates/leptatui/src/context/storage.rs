//! Thread-local render-scope context storage.
//!
//! This module stores type-erased context values in a stack of render frames so
//! descendant component scopes can shadow ancestor values.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
};

/// Stored context value erased behind its Rust type identifier.
type ContextValue = Box<dyn Any + Send + Sync>;
/// Single render-scope frame containing context values keyed by type.
type ContextFrame = HashMap<TypeId, ContextValue>;

thread_local! {
    /// Stack of active Leptatui render context frames for the current thread.
    static CONTEXT_STACK: RefCell<Vec<ContextFrame>> = RefCell::new(Vec::new());
}

/// Pushes a new empty context frame onto the current thread's stack.
pub(super) fn push_frame() {
    CONTEXT_STACK.with(|stack| stack.borrow_mut().push(HashMap::new()));
}

/// Pops the active context frame from the current thread's stack.
///
/// # Returns
///
/// A [`bool`] indicating whether a frame was available to pop.
pub(super) fn pop_frame() -> bool {
    CONTEXT_STACK.with(|stack| stack.borrow_mut().pop().is_some())
}

/// Stores a context value in the current Leptatui render scope.
///
/// # Arguments
///
/// * `value` — Context value to store in the top context frame.
///
/// # Returns
///
/// A [`Result`] containing `()` when a Leptatui context scope is active, or the
/// original value when no Leptatui scope exists.
pub(super) fn provide<T>(value: T) -> Result<(), T>
where
    T: Send + Sync + 'static,
{
    CONTEXT_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        let Some(frame) = stack.last_mut() else {
            return Err(value);
        };

        frame.insert(TypeId::of::<T>(), Box::new(value));
        Ok(())
    })
}

/// Returns a typed context value from the Leptatui context stack.
///
/// # Returns
///
/// An [`Option<T>`] containing the nearest scoped context value of type `T`.
pub(super) fn get<T>() -> Option<T>
where
    T: Clone + 'static,
{
    CONTEXT_STACK.with(|stack| {
        stack.borrow().iter().rev().find_map(|frame| {
            frame
                .get(&TypeId::of::<T>())
                .and_then(|value| value.downcast_ref::<T>())
                .cloned()
        })
    })
}
