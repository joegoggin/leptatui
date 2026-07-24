//! Dynamic child storage for render-tree views.

use std::{cell::RefCell, fmt, rc::Rc};

use crossterm::event::{Event, KeyEvent};

use crate::{
    LayoutSize,
    app::{AppControl, Result},
    component::{FocusedControl, KeyControl, LayoutPhase, RenderCtx},
    view::core::measurement::AvailableSpace,
};

use crate::view::{AnyView, IntoView, View};

/// Shared dynamic child that preserves compatible child state between refreshes.
#[derive(Clone)]
pub struct DynamicView {
    inner: Rc<DynamicViewInner>,
}

/// Deferred dynamic view state shared by cloned dynamic boundaries.
struct DynamicViewInner {
    child: Box<dyn Fn() -> AnyView>,
    current: RefCell<Option<AnyView>>,
}

impl DynamicView {
    /// Creates a dynamic view boundary from a child-producing closure.
    pub(crate) fn new(child: impl Fn() -> AnyView + 'static) -> Self {
        Self {
            inner: Rc::new(DynamicViewInner {
                child: Box::new(child),
                current: RefCell::new(None),
            }),
        }
    }

    /// Refreshes the current child and reads it for the duration of `read`.
    pub(crate) fn with_view<R>(&self, read: impl FnOnce(&AnyView) -> R) -> R {
        self.refresh();

        let current = self.inner.current.borrow();
        read(current.as_ref().expect("dynamic view should be refreshed"))
    }

    /// Reads the current child without rebuilding it, initializing it if needed.
    ///
    /// # Arguments
    ///
    /// * `read` — Callback receiving the retained child.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `read`.
    fn with_current_view<R>(&self, read: impl FnOnce(&AnyView) -> R) -> R {
        if self.inner.current.borrow().is_none() {
            self.refresh();
        }
        let current = self.inner.current.borrow();
        read(
            current
                .as_ref()
                .expect("dynamic view should be initialized"),
        )
    }

    /// Mutates the current child without rebuilding it, initializing it if needed.
    ///
    /// # Arguments
    ///
    /// * `write` — Callback receiving the retained mutable child.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `write`.
    fn with_current_view_mut<R>(&self, write: impl FnOnce(&mut AnyView) -> R) -> R {
        if self.inner.current.borrow().is_none() {
            self.refresh();
        }
        let mut current = self.inner.current.borrow_mut();
        write(
            current
                .as_mut()
                .expect("dynamic view should be initialized"),
        )
    }

    /// Rebuilds the child view and reconciles compatible state from the previous child.
    fn refresh(&self) {
        let mut next = (self.inner.child)();
        let mut current = self.inner.current.borrow_mut();

        if let Some(previous) = current.as_ref() {
            next.reconcile_from(previous);
        }

        *current = Some(next);
    }
}

impl View for DynamicView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        if ctx.layout_phase() == LayoutPhase::Paint {
            self.with_current_view(|child| child.render(ctx))
        } else {
            self.with_view(|child| child.render(ctx))
        }
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        if ctx.layout_phase() == LayoutPhase::Inactive {
            self.with_view(|child| child.measure(known_dimensions, available_space, ctx))
        } else {
            self.with_current_view(|child| child.measure(known_dimensions, available_space, ctx))
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn __visit_layout_children(
        &self,
        ctx: &mut RenderCtx<'_, '_>,
        visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>),
    ) {
        if ctx.layout_phase() == LayoutPhase::Build {
            self.refresh();
        }
        self.with_current_view(|child| visitor(child, ctx));
    }

    fn reconcile(&mut self, previous: &dyn View) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.inner = previous.inner.clone();
        }
    }

    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        previous
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|previous| Rc::ptr_eq(&self.inner, &previous.inner))
    }

    fn __dispatch_event(&mut self, event: &Event) -> Result<AppControl> {
        self.with_current_view_mut(|child| child.__dispatch_event(event))
    }

    fn __dispatch_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        self.with_current_view_mut(|child| child.__dispatch_key_event(key))
    }

    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        self.with_current_view_mut(AnyView::__flush_pending_input)
    }

    fn __focusable_count(&self) -> usize {
        self.with_current_view(AnyView::__focusable_count)
    }

    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.with_current_view(|child| child.__focused_index_inner(index))
    }

    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        self.with_current_view_mut(|child| child.__set_focus_by_index_inner(target, index));
    }

    fn __focusable_index_at_position_inner(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<usize> {
        self.with_current_view(|child| {
            child.__focusable_index_at_position_inner(column, row, index)
        })
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.with_current_view(|child| child.__focused_button_span(ctx))
    }

    fn __activate_focused_button(&self) -> Result<Option<AppControl>> {
        self.with_current_view(AnyView::__activate_focused_button)
    }

    fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.with_current_view_mut(|child| child.__handle_focused_input_key(key))
    }

    fn __focused_control(&self) -> Option<FocusedControl> {
        self.with_current_view(AnyView::__focused_control)
    }

    fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.with_current_view_mut(|child| child.__handle_form_key(key))
    }

    fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
        self.with_current_view_mut(|child| child.__scroll_first_overflowing(delta))
    }

    fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        self.with_current_view_mut(AnyView::__scroll_first_overflowing_to_top)
    }

    fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        self.with_current_view_mut(AnyView::__scroll_first_overflowing_to_bottom)
    }

    fn __has_overflowing_scroll_target(&self) -> bool {
        self.with_current_view(AnyView::__has_overflowing_scroll_target)
    }

    fn __scroll_overflowing_at_position(&mut self, column: u16, row: u16, delta: i16) -> bool {
        self.with_current_view_mut(|child| {
            child.__scroll_overflowing_at_position(column, row, delta)
        })
    }

    fn __set_scroll_to_top_key_pending(&self, pending: bool) -> bool {
        self.with_current_view(|child| child.__set_scroll_to_top_key_pending(pending))
    }

    fn __take_scroll_to_top_key_pending(&self) -> bool {
        self.with_current_view(AnyView::__take_scroll_to_top_key_pending)
    }

    fn __focused_link_target(&self) -> Option<crate::LinkTarget> {
        self.with_current_view(AnyView::__focused_link_target)
    }

    fn __request_scroll_to_id(&mut self, id: &str) -> bool {
        self.with_current_view_mut(|child| child.__request_scroll_to_id(id))
    }

    fn __has_scroll_to_anchor_request(&self) -> bool {
        self.with_current_view(AnyView::__has_scroll_to_anchor_request)
    }

    fn __navigate_markdown_history(&mut self, back: bool) -> bool {
        self.with_current_view_mut(|child| child.__navigate_markdown_history(back))
    }

    fn __clear_hit_areas(&self) {
        self.with_current_view(AnyView::__clear_hit_areas);
    }
}

impl PartialEq for DynamicView {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for DynamicView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DynamicView")
            .finish_non_exhaustive()
    }
}

/// Creates a dynamic child boundary.
///
/// # Arguments
///
/// * `child` — Closure that rebuilds the current child during traversal.
///
/// # Returns
///
/// A [`DynamicView`] retaining compatible child state between refreshes.
pub fn dynamic<V>(child: impl Fn() -> V + 'static) -> DynamicView
where
    V: IntoView,
{
    DynamicView::new(move || child().into_view())
}
