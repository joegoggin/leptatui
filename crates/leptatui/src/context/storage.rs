//! Thread-local render-scope context storage.
//!
//! This module stores type-erased context values in a stack of render frames so
//! descendant component scopes can shadow ancestor values.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

/// Stored context value erased behind its Rust type identifier.
type ContextValue = Box<dyn Any + Send + Sync>;
/// Single render-scope frame containing context values keyed by type.
type ContextFrameValues = HashMap<TypeId, ContextValue>;
pub(super) type ContextFrame = Rc<RefCell<ContextFrameValues>>;

thread_local! {
    /// Stack of active Leptatui render context frames for the current thread.
    static CONTEXT_STACK: RefCell<Vec<ContextFrame>> = RefCell::new(Vec::new());
}

/// Creates an empty reusable context frame.
pub(super) fn new_frame() -> ContextFrame {
    Rc::new(RefCell::new(HashMap::new()))
}

/// Clears values stored in a reusable context frame.
pub(super) fn clear_frame(frame: &ContextFrame) {
    frame.borrow_mut().clear();
}

/// Pushes an existing context frame onto the current thread's active stack.
pub(super) fn push_frame(frame: &ContextFrame) {
    CONTEXT_STACK.with(|stack| stack.borrow_mut().push(Rc::clone(frame)));
}

/// Pops the active context frame from the current thread's stack.
///
/// # Returns
///
/// A [`bool`] indicating whether a frame was available to pop.
pub(super) fn pop_frame() -> bool {
    CONTEXT_STACK.with(|stack| stack.borrow_mut().pop().is_some())
}

/// Returns whether the current thread has an active Leptatui context frame.
pub(super) fn has_active_frame() -> bool {
    CONTEXT_STACK.with(|stack| !stack.borrow().is_empty())
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
        let stack = stack.borrow();
        let Some(frame) = stack.last() else {
            return Err(value);
        };

        frame
            .borrow_mut()
            .insert(TypeId::of::<T>(), Box::new(value));
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
                .borrow()
                .get(&TypeId::of::<T>())
                .and_then(|value| value.downcast_ref::<T>())
                .cloned()
        })
    })
}
