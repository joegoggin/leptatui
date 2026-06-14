//! Component-boundary storage for render-tree views.
//!
//! This module wraps component values so they can live inside a cloneable view
//! tree while preserving component state and render-scope context between
//! events.

use std::{any::TypeId, cell::RefCell, fmt, rc::Rc};

use crossterm::event::{Event, KeyEvent};

use crate::{
    app::{AppControl, Result},
    component::{Component, KeyControl, RenderCtx},
    context::ContextScope,
};

/// Shared component boundary stored inside a render tree.
#[derive(Clone)]
pub struct ComponentView {
    inner: Rc<ComponentViewInner>,
}

/// Component boundary state shared by cloned views.
struct ComponentViewInner {
    /// Concrete component type represented by this boundary.
    component_type: TypeId,
    /// Shared mutable component storage, populated lazily for `view!` tags.
    component: RefCell<Option<Rc<RefCell<dyn Component>>>>,
    /// Deferred constructor for lazy component tags.
    factory: RefCell<Option<Box<dyn FnOnce() -> Rc<RefCell<dyn Component>>>>>,
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
                component: RefCell::new(Some(Rc::new(RefCell::new(component)))),
                factory: RefCell::new(None),
                context: ContextScope::new(),
            }),
        }
    }

    /// Creates a lazy component boundary from a component constructor.
    pub(crate) fn new_factory<C>(factory: impl FnOnce() -> C + 'static) -> Self
    where
        C: Component + 'static,
    {
        Self {
            inner: Rc::new(ComponentViewInner {
                component_type: TypeId::of::<C>(),
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
        let component = self.component();

        self.inner
            .context
            .with_reset(|| component.borrow_mut().render(ctx))
    }

    /// Returns the minimum useful render height inside this component boundary.
    pub(crate) fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        let component = self.component();

        self.inner
            .context
            .with_reset(|| component.borrow().__min_height(ctx))
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
        let component = self.component();

        self.inner
            .context
            .with(|| component.borrow_mut().handle_event(event))
    }

    /// Dispatches a key event through custom handlers only.
    #[doc(hidden)]
    pub(crate) fn dispatch_key_event(&self, key: KeyEvent) -> Result<KeyControl> {
        let component = self.component();

        self.inner
            .context
            .with(|| component.borrow_mut().__dispatch_key_event(key))
    }

    /// Returns the number of focusable controls inside the component boundary.
    #[doc(hidden)]
    pub(crate) fn focusable_count(&self) -> usize {
        let component = self.component();

        self.inner
            .context
            .with(|| component.borrow().__focusable_count())
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    pub(crate) fn focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        let component = self.component();

        self.inner
            .context
            .with(|| component.borrow().__focused_index_inner(index))
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    pub(crate) fn set_focus_by_index_inner(&self, target: usize, index: &mut usize) {
        let component = self.component();

        self.inner.context.with(|| {
            component
                .borrow_mut()
                .__set_focus_by_index_inner(target, index)
        });
    }

    /// Returns the focused control's vertical span inside this component boundary.
    #[doc(hidden)]
    pub(crate) fn focused_button_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        let component = self.component();

        self.inner
            .context
            .with(|| component.borrow().__focused_button_span(ctx))
    }

    /// Activates the focused control inside the component boundary, if any.
    #[doc(hidden)]
    pub(crate) fn activate_focused_button(&self) -> Option<AppControl> {
        let component = self.component();

        self.inner
            .context
            .with(|| component.borrow().__activate_focused_button())
    }

    /// Scrolls the first overflowing layout inside this component boundary.
    #[doc(hidden)]
    pub(crate) fn scroll_first_overflowing(&self, delta: i16) -> bool {
        let component = self.component();

        self.inner
            .context
            .with(|| component.borrow_mut().__scroll_first_overflowing(delta))
    }

    /// Scrolls the first overflowing layout inside this component boundary to the top.
    #[doc(hidden)]
    pub(crate) fn scroll_first_overflowing_to_top(&self) -> bool {
        let component = self.component();

        self.inner
            .context
            .with(|| component.borrow_mut().__scroll_first_overflowing_to_top())
    }

    /// Scrolls the first overflowing layout inside this component boundary to the bottom.
    #[doc(hidden)]
    pub(crate) fn scroll_first_overflowing_to_bottom(&self) -> bool {
        let component = self.component();

        self.inner.context.with(|| {
            component
                .borrow_mut()
                .__scroll_first_overflowing_to_bottom()
        })
    }

    /// Returns whether this component boundary contains an overflowing scroll target.
    #[doc(hidden)]
    pub(crate) fn has_overflowing_scroll_target(&self) -> bool {
        let component = self.component();

        self.inner
            .context
            .with(|| component.borrow().__has_overflowing_scroll_target())
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

    /// Returns whether two boundaries represent the same concrete component type.
    pub(crate) fn is_same_component_type(&self, other: &Self) -> bool {
        self.inner.component_type == other.inner.component_type
    }

    /// Returns the shared mutable component, materializing lazy boundaries once.
    fn component(&self) -> Rc<RefCell<dyn Component>> {
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
