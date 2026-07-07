//! Stylesheet registration for generated components.
//!
//! This module stores stylesheets registered during `#[component]` setup and
//! exposes the hidden hook used by the `stylesheet!` macro.

use std::{cell::RefCell, rc::Rc};

use crate::style::Stylesheet;

thread_local! {
    /// Stack of active generated-component stylesheet registries.
    static STYLESHEET_STACK: RefCell<Vec<StylesheetRegistry>> = const { RefCell::new(Vec::new()) };
}

/// Shared stylesheet registry owned during generated component setup.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct StylesheetRegistry {
    /// Stylesheet collected while generated component setup runs.
    stylesheet: Rc<RefCell<Stylesheet>>,
}

impl StylesheetRegistry {
    /// Creates an empty stylesheet registry.
    #[doc(hidden)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the stylesheet collected during component setup.
    #[doc(hidden)]
    pub fn stylesheet(&self) -> Stylesheet {
        self.stylesheet.borrow().clone()
    }

    /// Extends the collected stylesheet with another stylesheet.
    ///
    /// # Arguments
    ///
    /// * `stylesheet` — Stylesheet rules to append to this registry.
    fn register(&self, stylesheet: &Stylesheet) {
        self.stylesheet.borrow_mut().extend(stylesheet);
    }
}

/// Registers a stylesheet for the current generated component, if any.
#[doc(hidden)]
pub fn __register_stylesheet(stylesheet: &Stylesheet) {
    if let Some(registry) = STYLESHEET_STACK.with(|stack| stack.borrow().last().cloned()) {
        registry.register(stylesheet);
    }
}

/// Runs component setup with a stylesheet registry active.
#[doc(hidden)]
pub fn __with_stylesheet_registry<R>(
    registry: &StylesheetRegistry,
    setup: impl FnOnce() -> R,
) -> R {
    let _guard = StylesheetRegistryGuard::enter(registry);
    setup()
}

/// Scope guard that pops the active stylesheet registry on drop.
struct StylesheetRegistryGuard;

impl StylesheetRegistryGuard {
    /// Pushes a stylesheet registry onto the active setup stack.
    ///
    /// # Arguments
    ///
    /// * `registry` — Registry to expose during component setup.
    ///
    /// # Returns
    ///
    /// A [`StylesheetRegistryGuard`] that restores the previous stack on drop.
    fn enter(registry: &StylesheetRegistry) -> Self {
        STYLESHEET_STACK.with(|stack| stack.borrow_mut().push(registry.clone()));
        Self
    }
}

impl Drop for StylesheetRegistryGuard {
    /// Pops the stylesheet registry stack.
    fn drop(&mut self) {
        let popped = STYLESHEET_STACK.with(|stack| stack.borrow_mut().pop().is_some());
        debug_assert!(popped, "stylesheet registry stack underflow");
    }
}
