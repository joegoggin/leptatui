//! Keyboard event hooks for generated components.
//!
//! This module stores key handlers registered during `#[component]` setup and
//! exposes the public hook used by application code.

use std::{cell::RefCell, rc::Rc};

use crossterm::event::{KeyEvent, KeyEventKind};

use crate::app::AppControl;

/// Controls key-event propagation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyControl {
    /// This handler did not handle the key.
    Pass,
    /// This handler handled the key and the app should continue running.
    Handled,
    /// This handler handled the key and the app should exit.
    Exit,
}

impl From<KeyControl> for AppControl {
    /// Converts key propagation control into app-loop control.
    fn from(control: KeyControl) -> Self {
        match control {
            KeyControl::Pass | KeyControl::Handled => Self::Continue,
            KeyControl::Exit => Self::Exit,
        }
    }
}

impl From<AppControl> for KeyControl {
    /// Converts app-loop control from default button actions into key control.
    fn from(control: AppControl) -> Self {
        match control {
            AppControl::Continue => Self::Handled,
            AppControl::Exit => Self::Exit,
        }
    }
}

/// Callback invoked for matching key events.
type KeyHandlerCallback = Box<dyn FnMut(KeyEvent) -> KeyControl>;

/// Registered key handler and the event kind it accepts.
struct KeyHandler {
    /// Key event kind required before invoking the callback.
    kind: KeyEventKind,
    /// Callback invoked when the key kind matches.
    callback: KeyHandlerCallback,
}

thread_local! {
    /// Stack of active generated-component key registries.
    static KEY_HANDLER_STACK: RefCell<Vec<KeyHandlerRegistry>> = const { RefCell::new(Vec::new()) };
}

/// Shared key-handler registry owned by a generated component instance.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct KeyHandlerRegistry {
    /// Registered handlers shared with generated component setup.
    handlers: Rc<RefCell<Vec<KeyHandler>>>,
}

impl KeyHandlerRegistry {
    /// Creates an empty key-handler registry.
    #[doc(hidden)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Dispatches a key through registered handlers in source order.
    #[doc(hidden)]
    pub fn handle(&self, key: KeyEvent) -> KeyControl {
        for handler in self.handlers.borrow_mut().iter_mut() {
            if handler.kind != key.kind {
                continue;
            }

            let control = (handler.callback)(key);
            if control != KeyControl::Pass {
                return control;
            }
        }

        KeyControl::Pass
    }

    /// Registers a callback for one key event kind.
    ///
    /// # Arguments
    ///
    /// * `kind` — Key event kind that should invoke the callback.
    /// * `callback` — Handler callback to run for matching key events.
    fn register(&self, kind: KeyEventKind, callback: KeyHandlerCallback) {
        self.handlers
            .borrow_mut()
            .push(KeyHandler { kind, callback });
    }
}

/// Registers a key-event handler for the current generated component.
///
/// Handlers run after descendant components have had a chance to handle the
/// key and before built-in button focus or activation behavior. Return
/// [`KeyControl::Handled`] to stop propagation while keeping the app running,
/// [`KeyControl::Exit`] to exit the app, or [`KeyControl::Pass`] to let other
/// handlers and defaults try the key.
///
/// The handler only runs for key events whose kind matches `kind`.
///
/// # Arguments
///
/// * `kind` — Key event kind that should invoke the handler.
/// * `handler` — Callback that handles matching key events.
///
/// # Panics
///
/// Panics when called outside `#[component]` setup.
pub fn use_key_event<F>(kind: KeyEventKind, handler: F)
where
    F: FnMut(KeyEvent) -> KeyControl + 'static,
{
    let registry = KEY_HANDLER_STACK.with(|stack| stack.borrow().last().cloned());
    let Some(registry) = registry else {
        panic!("use_key_event can only be called while a #[component] is being created");
    };

    registry.register(kind, Box::new(handler));
}

/// Runs component setup with a key-handler registry active.
#[doc(hidden)]
pub fn __with_key_handler_registry<R>(
    registry: &KeyHandlerRegistry,
    setup: impl FnOnce() -> R,
) -> R {
    let _guard = KeyHandlerRegistryGuard::enter(registry);
    setup()
}

/// Scope guard that pops the active key-handler registry on drop.
struct KeyHandlerRegistryGuard;

impl KeyHandlerRegistryGuard {
    /// Pushes a key-handler registry onto the active setup stack.
    ///
    /// # Arguments
    ///
    /// * `registry` — Registry to expose during component setup.
    ///
    /// # Returns
    ///
    /// A [`KeyHandlerRegistryGuard`] that restores the previous stack on drop.
    fn enter(registry: &KeyHandlerRegistry) -> Self {
        KEY_HANDLER_STACK.with(|stack| stack.borrow_mut().push(registry.clone()));
        Self
    }
}

impl Drop for KeyHandlerRegistryGuard {
    /// Pops the key-handler registry stack.
    fn drop(&mut self) {
        let popped = KEY_HANDLER_STACK.with(|stack| stack.borrow_mut().pop().is_some());
        debug_assert!(popped, "key handler registry stack underflow");
    }
}
