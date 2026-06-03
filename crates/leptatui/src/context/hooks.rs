//! Public context hooks.

use std::any::type_name;

use super::{scope::ContextScope, storage};

/// Provides a typed context value to descendant component render scopes.
///
/// Falls back to Leptos owner context storage when no Leptatui render scope is
/// active.
///
/// # Arguments
///
/// * `value` — Context value to store by its concrete Rust type.
pub fn provide_context<T>(value: T)
where
    T: Send + Sync + 'static,
{
    if let Err(value) = storage::provide(value) {
        leptos::context::provide_context(value);
    }
}

/// Returns a typed context value from the nearest render scope that provides it.
///
/// Falls back to Leptos owner context lookup when no matching Leptatui context
/// value is present.
///
/// # Returns
///
/// An [`Option<T>`] containing the nearest context value of type `T`.
pub fn use_context<T>() -> Option<T>
where
    T: Clone + 'static,
{
    storage::get::<T>().or_else(leptos::context::use_context::<T>)
}

/// Returns a typed context value or panics with a clear missing-context error.
///
/// # Returns
///
/// A `T` context value cloned from the nearest matching context provider.
///
/// # Panics
///
/// Panics if no Leptatui or Leptos owner context value of type `T` exists.
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

/// Runs a closure inside a fresh Leptatui context scope.
///
/// # Arguments
///
/// * `render` — Closure that can provide and read scoped context values.
///
/// # Returns
///
/// An `R` value returned by `render`.
#[doc(hidden)]
pub fn __with_context_scope<R>(render: impl FnOnce() -> R) -> R {
    ContextScope::new().with(render)
}

#[cfg(test)]
/// Unit tests for Leptatui context stack behavior.
mod tests {
    use super::*;

    /// Verifies descendant scopes can read context from ancestor scopes.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// __with_context_scope(|| {
    ///     provide_context(String::from("ancestor"));
    ///     __with_context_scope(|| use_context::<String>())
    /// })
    /// ```
    ///
    /// # Assertions
    ///
    /// - The inner scope reads the ancestor string context.
    #[test]
    fn descendant_scope_reads_ancestor_context() {
        __with_context_scope(|| {
            provide_context(String::from("ancestor"));

            __with_context_scope(|| {
                assert_eq!(use_context::<String>().as_deref(), Some("ancestor"));
            });
        });
    }

    /// Verifies inner scopes shadow ancestor context and restore it on exit.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// provide_context(String::from("ancestor"));
    /// __with_context_scope(|| provide_context(String::from("child")));
    /// ```
    ///
    /// # Assertions
    ///
    /// - The inner scope reads `child`.
    /// - The outer scope reads `ancestor` after the inner scope exits.
    ///
    /// # Why
    ///
    /// Render scopes should isolate descendant overrides without permanently
    /// replacing ancestor-provided values.
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

    /// Verifies missing Leptatui context returns `None`.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// __with_context_scope(|| use_context::<String>())
    /// ```
    ///
    /// # Assertions
    ///
    /// - The lookup returns `None`.
    #[test]
    fn use_context_returns_none_when_missing() {
        __with_context_scope(|| {
            assert_eq!(use_context::<String>(), None);
        });
    }

    /// Verifies missing required context includes the requested type name.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// __with_context_scope(|| expect_context::<String>())
    /// ```
    ///
    /// # Assertions
    ///
    /// - The lookup panics.
    /// - The panic message contains the missing-context diagnostic.
    /// - The panic message contains `String`.
    ///
    /// # Why
    ///
    /// Required context failures should point to the missing Rust type.
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
