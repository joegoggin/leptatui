//! Typed context APIs used by Leptatui apps.
//!
//! Context values are keyed by Rust type and are visible to descendant
//! Leptatui component render scopes. Values provided in an inner scope shadow
//! values of the same type from ancestor scopes.

use std::{
    any::{Any, TypeId, type_name},
    cell::RefCell,
    collections::HashMap,
};

type ContextValue = Box<dyn Any + Send + Sync>;
type ContextFrame = HashMap<TypeId, ContextValue>;

thread_local! {
    static CONTEXT_STACK: RefCell<Vec<ContextFrame>> = RefCell::new(Vec::new());
}

/// Provides a typed context value to descendant component render scopes.
pub fn provide_context<T>(value: T)
where
    T: Send + Sync + 'static,
{
    if let Err(value) = provide_leptatui_context(value) {
        leptos::context::provide_context(value);
    }
}

fn provide_leptatui_context<T>(value: T) -> Result<(), T>
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

/// Returns a typed context value from the nearest render scope that provides it.
pub fn use_context<T>() -> Option<T>
where
    T: Clone + 'static,
{
    use_leptatui_context::<T>().or_else(leptos::context::use_context::<T>)
}

/// Returns a typed context value or panics with a clear missing-context error.
#[track_caller]
pub fn expect_context<T>() -> T
where
    T: Clone + 'static,
{
    use_context::<T>().unwrap_or_else(|| {
        panic!(
            "missing Leptatui context value for type `{}`",
            type_name::<T>()
        )
    })
}

#[doc(hidden)]
pub fn __with_context_scope<R>(render: impl FnOnce() -> R) -> R {
    let _scope = ContextScopeGuard::enter();
    render()
}

fn use_leptatui_context<T>() -> Option<T>
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

struct ContextScopeGuard;

impl ContextScopeGuard {
    fn enter() -> Self {
        CONTEXT_STACK.with(|stack| stack.borrow_mut().push(HashMap::new()));
        Self
    }
}

impl Drop for ContextScopeGuard {
    fn drop(&mut self) {
        CONTEXT_STACK.with(|stack| {
            let popped = stack.borrow_mut().pop();
            debug_assert!(popped.is_some(), "context scope stack underflow");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descendant_scope_reads_ancestor_context() {
        __with_context_scope(|| {
            provide_context(String::from("ancestor"));

            __with_context_scope(|| {
                assert_eq!(use_context::<String>().as_deref(), Some("ancestor"));
            });
        });
    }

    #[test]
    fn inner_scope_shadows_and_then_restores_context() {
        __with_context_scope(|| {
            provide_context(String::from("ancestor"));

            __with_context_scope(|| {
                provide_context(String::from("child"));
                assert_eq!(expect_context::<String>(), "child");
            });

            assert_eq!(expect_context::<String>(), "ancestor");
        });
    }

    #[test]
    fn use_context_returns_none_when_missing() {
        __with_context_scope(|| {
            assert_eq!(use_context::<String>(), None);
        });
    }

    #[test]
    fn expect_context_panics_with_missing_type_name() {
        let panic = std::panic::catch_unwind(|| {
            __with_context_scope(|| {
                let _ = expect_context::<String>();
            });
        })
        .expect_err("expect_context should panic when context is missing");

        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("panic payload should be a string");

        assert!(message.contains("missing Leptatui context value"));
        assert!(message.contains("String"));
    }
}
