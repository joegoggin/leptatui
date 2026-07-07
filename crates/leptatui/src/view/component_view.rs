//! Component-boundary storage for render-tree views.
//!
//! This module wraps component values so they can live inside a cloneable view
//! tree while preserving component state and render-scope context between
//! events.

use std::{any::TypeId, cell::RefCell, fmt, rc::Rc};

use crossterm::event::{Event, KeyEvent};

use crate::{
    app::{AppControl, Result},
    component::{Component, FocusedControl, KeyControl, RenderCtx},
    context::ContextScope,
};

/// Shared mutable component instance stored by a component view boundary.
type SharedComponent = Rc<RefCell<dyn Component>>;
/// Lazy factory that creates a shared component instance on first use.
type ComponentFactory = Box<dyn FnOnce() -> SharedComponent>;

/// Shared component boundary stored inside a render tree.
#[derive(Clone)]
pub struct ComponentView {
    inner: Rc<ComponentViewInner>,
}

/// Component boundary state shared by cloned views.
struct ComponentViewInner {
    /// Concrete component type represented by this boundary.
    component_type: TypeId,
    /// Whether reconciliation may preserve this boundary by component type.
    preserve_on_reconcile: bool,
    /// Shared mutable component storage, populated lazily for `view!` tags.
    component: RefCell<Option<SharedComponent>>,
    /// Deferred constructor for lazy component tags.
    factory: RefCell<Option<ComponentFactory>>,
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
    pub(crate) fn new<C>(component: C) -> Self
    where
        C: Component + 'static,
    {
        Self {
            inner: Rc::new(ComponentViewInner {
                component_type: TypeId::of::<C>(),
                preserve_on_reconcile: false,
                component: RefCell::new(Some(Rc::new(RefCell::new(component)))),
                factory: RefCell::new(None),
                context: ContextScope::new(),
            }),
        }
    }

    /// Creates a lazy component boundary from a component constructor.
    pub(crate) fn new_factory<C>(
        preserve_on_reconcile: bool,
        factory: impl FnOnce() -> C + 'static,
    ) -> Self
    where
        C: Component + 'static,
    {
        Self {
            inner: Rc::new(ComponentViewInner {
                component_type: TypeId::of::<C>(),
                preserve_on_reconcile,
                component: RefCell::new(None),
                factory: RefCell::new(Some(Box::new(move || Rc::new(RefCell::new(factory()))))),
                context: ContextScope::new(),
            }),
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
        self.with_reset_component_mut(|component| component.render(ctx))
    }

    /// Returns the minimum useful render height inside this component boundary.
    pub(crate) fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        self.with_reset_component(|component| component.__min_height(ctx))
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
        self.with_component_mut(|component| component.handle_event(event))
    }

    /// Dispatches a key event through custom handlers only.
    #[doc(hidden)]
    pub(crate) fn dispatch_key_event(&self, key: KeyEvent) -> Result<KeyControl> {
        self.with_component_mut(|component| component.__dispatch_key_event(key))
    }

    /// Returns the number of focusable controls inside the component boundary.
    #[doc(hidden)]
    pub(crate) fn focusable_count(&self) -> usize {
        self.with_component(|component| component.__focusable_count())
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    pub(crate) fn focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.with_component(|component| component.__focused_index_inner(index))
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    pub(crate) fn set_focus_by_index_inner(&self, target: usize, index: &mut usize) {
        self.with_component_mut(|component| component.__set_focus_by_index_inner(target, index));
    }

    /// Returns the focused control's vertical span inside this component boundary.
    #[doc(hidden)]
    pub(crate) fn focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.with_component(|component| component.__focused_button_span(ctx))
    }

    /// Activates the focused control inside the component boundary, if any.
    #[doc(hidden)]
    pub(crate) fn activate_focused_button(&self) -> Option<AppControl> {
        self.with_component(|component| component.__activate_focused_button())
    }

    /// Handles a key on the focused input inside this component boundary, if any.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to forward to the component boundary.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the key control result when an input handles
    /// the key.
    #[doc(hidden)]
    pub(crate) fn handle_focused_input_key(&self, key: KeyEvent) -> Option<KeyControl> {
        self.with_component_mut(|component| component.__handle_focused_input_key(key))
    }

    /// Emits any expired pending insert-mode key inside this component boundary.
    #[doc(hidden)]
    pub(crate) fn flush_pending_input(&self) -> Option<AppControl> {
        self.with_component_mut(|component| component.__flush_pending_input())
    }

    /// Returns the focused built-in control inside this component boundary.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing focused control metadata when a supported
    /// built-in control is focused.
    #[doc(hidden)]
    pub(crate) fn focused_control(&self) -> Option<FocusedControl> {
        self.with_component(|component| component.__focused_control())
    }

    /// Handles form-owned submit or cancel keys inside this component boundary.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to evaluate for nested form behavior.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing key traversal control when a nested form
    /// handles the key.
    #[doc(hidden)]
    pub(crate) fn handle_form_key(&self, key: KeyEvent) -> Option<KeyControl> {
        self.with_component_mut(|component| component.__handle_form_key(key))
    }

    /// Scrolls the first overflowing layout inside this component boundary.
    #[doc(hidden)]
    pub(crate) fn scroll_first_overflowing(&self, delta: i16) -> bool {
        self.with_component_mut(|component| component.__scroll_first_overflowing(delta))
    }

    /// Scrolls the first overflowing layout inside this component boundary to the top.
    #[doc(hidden)]
    pub(crate) fn scroll_first_overflowing_to_top(&self) -> bool {
        self.with_component_mut(|component| component.__scroll_first_overflowing_to_top())
    }

    /// Scrolls the first overflowing layout inside this component boundary to the bottom.
    #[doc(hidden)]
    pub(crate) fn scroll_first_overflowing_to_bottom(&self) -> bool {
        self.with_component_mut(|component| component.__scroll_first_overflowing_to_bottom())
    }

    /// Returns whether this component boundary contains an overflowing scroll target.
    #[doc(hidden)]
    pub(crate) fn has_overflowing_scroll_target(&self) -> bool {
        self.with_component(|component| component.__has_overflowing_scroll_target())
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

    /// Returns whether reconciliation may preserve these component boundaries.
    pub(crate) fn can_reconcile_from(&self, other: &Self) -> bool {
        self.inner.preserve_on_reconcile
            && other.inner.preserve_on_reconcile
            && self.inner.component_type == other.inner.component_type
    }

    /// Reads the materialized component inside its persistent context scope.
    fn with_component<R>(&self, read: impl FnOnce(&dyn Component) -> R) -> R {
        let component = self.component();

        self.inner.context.with(|| {
            let component = component.borrow();
            read(&*component)
        })
    }

    /// Mutates the materialized component inside its persistent context scope.
    fn with_component_mut<R>(&self, write: impl FnOnce(&mut dyn Component) -> R) -> R {
        let component = self.component();

        self.inner.context.with(|| {
            let mut component = component.borrow_mut();
            write(&mut *component)
        })
    }

    /// Reads the materialized component inside a reset context scope.
    fn with_reset_component<R>(&self, read: impl FnOnce(&dyn Component) -> R) -> R {
        let component = self.component();

        self.inner.context.with_reset(|| {
            let component = component.borrow();
            read(&*component)
        })
    }

    /// Mutates the materialized component inside a reset context scope.
    fn with_reset_component_mut<R>(&self, write: impl FnOnce(&mut dyn Component) -> R) -> R {
        let component = self.component();

        self.inner.context.with_reset(|| {
            let mut component = component.borrow_mut();
            write(&mut *component)
        })
    }

    /// Returns the shared mutable component, materializing lazy boundaries once.
    fn component(&self) -> SharedComponent {
        if let Some(component) = self.inner.component.borrow().as_ref() {
            return Rc::clone(component);
        }

        let factory = self
            .inner
            .factory
            .borrow_mut()
            .take()
            .expect("lazy component factory should be available");
        let component = factory();

        *self.inner.component.borrow_mut() = Some(component.clone());
        component
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
