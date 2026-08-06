//! Reactive dynamic child storage for render-tree views.

use std::{
    cell::RefCell,
    fmt,
    rc::Rc,
    sync::{Arc, RwLock, Weak},
};

use crossterm::event::{Event, KeyEvent};
use leptos::prelude::untrack;
use leptos::reactive::graph::{
    AnySource, AnySubscriber, ReactiveNode, ReactiveNodeState, Source, Subscriber, ToAnySubscriber,
    WithObserver,
};

use crate::{
    LayoutSize,
    app::{AppControl, Result, request_redraw},
    component::{FocusedControl, KeyControl, LayoutPhase, RenderCtx},
    view::core::measurement::AvailableSpace,
};

use crate::view::{AnyView, IntoView, View};

/// Mutation applied to each child produced by a dynamic text-like element.
type DynamicConfigurator = Box<dyn Fn(&mut AnyView)>;

/// Shared dynamic child that preserves compatible child state between refreshes.
#[derive(Clone)]
pub struct DynamicView {
    inner: Rc<DynamicViewInner>,
}

/// Deferred dynamic view state shared by cloned dynamic boundaries.
struct DynamicViewInner {
    child: Box<dyn Fn() -> AnyView>,
    invalidation: DynamicInvalidation,
    reconcile_on_refresh: bool,
    current: RefCell<Option<AnyView>>,
    subscriber: DynamicSubscriber,
    configurators: RefCell<Vec<DynamicConfigurator>>,
}

/// Dependency policy used by one dynamic boundary.
enum DynamicInvalidation {
    /// Rebuilds when any signal read by the child factory changes.
    Child,
    /// Rebuilds without reconciliation when a tracked key changes value.
    Key(Box<dyn Fn() -> bool>),
}

/// Thread-safe reactive subscriber that wakes the terminal app when invalidated.
#[derive(Clone)]
struct DynamicSubscriber {
    inner: Arc<DynamicSubscriberInner>,
}

/// Mutable dependency state behind a dynamic subscriber.
struct DynamicSubscriberInner {
    state: RwLock<DynamicSubscriberState>,
}

/// Latest invalidation state and the reactive sources observed by a boundary.
struct DynamicSubscriberState {
    node: ReactiveNodeState,
    sources: Vec<AnySource>,
}

impl DynamicSubscriber {
    /// Creates a clean subscriber for a boundary with no retained child.
    ///
    /// # Returns
    ///
    /// A [`DynamicSubscriber`] ready to collect dependencies during the
    /// boundary's initial child construction.
    fn new() -> Self {
        Self {
            inner: Arc::new(DynamicSubscriberInner {
                state: RwLock::new(DynamicSubscriberState {
                    node: ReactiveNodeState::Clean,
                    sources: Vec::new(),
                }),
            }),
        }
    }

    /// Returns whether this boundary needs to reevaluate its observed closure.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether a tracked dependency changed.
    fn update_if_necessary(&self) -> bool {
        self.to_any_subscriber()
            .with_observer(|| self.inner.update_if_necessary())
    }

    /// Replaces prior dependencies with those read by `evaluate`.
    ///
    /// # Arguments
    ///
    /// * `evaluate` — Closure whose reactive reads should be observed.
    ///
    /// # Returns
    ///
    /// A `T` value returned by `evaluate`.
    fn track<T>(&self, evaluate: impl FnOnce() -> T) -> T {
        let subscriber = self.to_any_subscriber();
        self.inner.clear_sources(&subscriber);
        subscriber.with_observer(evaluate)
    }

    /// Removes this subscriber from every source it currently observes.
    fn dispose(&self) {
        let subscriber = self.to_any_subscriber();
        self.inner.clear_sources(&subscriber);
    }
}

impl ToAnySubscriber for DynamicSubscriber {
    fn to_any_subscriber(&self) -> AnySubscriber {
        let inner: Arc<dyn Subscriber + Send + Sync> = self.inner.clone();
        AnySubscriber(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&inner) as Weak<dyn Subscriber + Send + Sync>,
        )
    }
}

impl ReactiveNode for DynamicSubscriberInner {
    fn mark_dirty(&self) {
        self.state
            .write()
            .expect("dynamic subscriber poisoned")
            .node = ReactiveNodeState::Dirty;
        request_redraw();
    }

    fn mark_check(&self) {
        let mut state = self.state.write().expect("dynamic subscriber poisoned");
        if state.node == ReactiveNodeState::Clean {
            state.node = ReactiveNodeState::Check;
        }
        drop(state);
        request_redraw();
    }

    fn mark_subscribers_check(&self) {}

    fn update_if_necessary(&self) -> bool {
        let mut state = self.state.write().expect("dynamic subscriber poisoned");
        match state.node {
            ReactiveNodeState::Clean => false,
            ReactiveNodeState::Dirty => {
                state.node = ReactiveNodeState::Clean;
                true
            }
            ReactiveNodeState::Check => {
                state.node = ReactiveNodeState::Clean;
                let sources = state.sources.clone();
                drop(state);
                sources
                    .into_iter()
                    .any(|source| source.update_if_necessary())
            }
        }
    }
}

impl Subscriber for DynamicSubscriberInner {
    fn add_source(&self, source: AnySource) {
        let mut state = self.state.write().expect("dynamic subscriber poisoned");
        if !state.sources.contains(&source) {
            state.sources.push(source);
        }
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        let sources = std::mem::take(
            &mut self
                .state
                .write()
                .expect("dynamic subscriber poisoned")
                .sources,
        );
        for source in sources {
            source.remove_subscriber(subscriber);
        }
    }
}

impl Drop for DynamicViewInner {
    fn drop(&mut self) {
        self.subscriber.dispose();
    }
}

impl DynamicView {
    /// Creates a dynamic view boundary from a child-producing closure.
    pub(crate) fn new(child: impl Fn() -> AnyView + 'static) -> Self {
        Self {
            inner: Rc::new(DynamicViewInner {
                child: Box::new(child),
                invalidation: DynamicInvalidation::Child,
                reconcile_on_refresh: true,
                current: RefCell::new(None),
                subscriber: DynamicSubscriber::new(),
                configurators: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Creates a dynamic boundary with explicit invalidation behavior.
    ///
    /// # Arguments
    ///
    /// * `child` — Callback building a replacement child.
    /// * `key_changed` — Tracked callback deciding whether to replace an existing child.
    /// * `reconcile_on_refresh` — Whether replacement children retain compatible state.
    ///
    /// # Returns
    ///
    /// A [`DynamicView`] governed by the supplied invalidation callbacks.
    fn new_with_invalidation(
        child: impl Fn() -> AnyView + 'static,
        key_changed: impl Fn() -> bool + 'static,
        reconcile_on_refresh: bool,
    ) -> Self {
        Self {
            inner: Rc::new(DynamicViewInner {
                child: Box::new(child),
                invalidation: DynamicInvalidation::Key(Box::new(key_changed)),
                reconcile_on_refresh,
                current: RefCell::new(None),
                subscriber: DynamicSubscriber::new(),
                configurators: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Applies an id to every child produced by this dynamic boundary.
    ///
    /// # Arguments
    ///
    /// * `id` — Id selector assigned to each produced child.
    ///
    /// # Returns
    ///
    /// This [`DynamicView`] with the child configurator installed.
    #[doc(hidden)]
    pub fn with_id(self, id: impl Into<String>) -> Self {
        let id = id.into();
        self.inner
            .configurators
            .borrow_mut()
            .push(Box::new(move |child| {
                if let Some(metadata) = child.style_metadata_mut() {
                    metadata.set_id(id.clone());
                }
            }));
        self
    }

    /// Applies classes to every child produced by this dynamic boundary.
    ///
    /// # Arguments
    ///
    /// * `classes` — Whitespace-separated classes assigned to each child.
    ///
    /// # Returns
    ///
    /// This [`DynamicView`] with the child configurator installed.
    #[doc(hidden)]
    pub fn with_classes(self, classes: impl Into<String>) -> Self {
        let classes = classes.into();
        self.inner
            .configurators
            .borrow_mut()
            .push(Box::new(move |child| {
                if let Some(metadata) = child.style_metadata_mut() {
                    metadata.set_classes(classes.clone());
                }
            }));
        self
    }

    /// Applies an inline style to every child produced by this dynamic boundary.
    ///
    /// # Arguments
    ///
    /// * `style` — Inline style assigned to each produced child.
    ///
    /// # Returns
    ///
    /// This [`DynamicView`] with the child configurator installed.
    #[doc(hidden)]
    pub fn with_inline_style(self, style: crate::TuiStyle) -> Self {
        self.inner
            .configurators
            .borrow_mut()
            .push(Box::new(move |child| {
                if let Some(metadata) = child.style_metadata_mut() {
                    metadata.set_inline_style(style.clone());
                }
            }));
        self
    }

    /// Applies table-cell alignment to every child produced by this boundary.
    ///
    /// # Arguments
    ///
    /// * `alignment` — Alignment assigned when the child is a table cell.
    ///
    /// # Returns
    ///
    /// This [`DynamicView`] with the child configurator installed.
    #[doc(hidden)]
    pub fn alignment(self, alignment: crate::CellAlignment) -> Self {
        self.inner
            .configurators
            .borrow_mut()
            .push(Box::new(move |child| {
                if let Some(cell) = child.downcast_mut::<crate::TableCellView>() {
                    cell.alignment = alignment;
                }
            }));
        self
    }

    /// Applies a code-block language to every child produced by this boundary.
    ///
    /// # Arguments
    ///
    /// * `language` — Syntax language assigned when the child is a code block.
    ///
    /// # Returns
    ///
    /// This [`DynamicView`] with the child configurator installed.
    #[doc(hidden)]
    pub fn language(self, language: impl Into<String>) -> Self {
        let language = language.into();
        self.inner
            .configurators
            .borrow_mut()
            .push(Box::new(move |child| {
                if let Some(code) = child.downcast_mut::<crate::CodeBlockView>() {
                    *code = code.clone().language(language.clone());
                }
            }));
        self
    }

    /// Applies line-number visibility to every child produced by this boundary.
    ///
    /// # Arguments
    ///
    /// * `line_numbers` — Whether produced code blocks show line numbers.
    ///
    /// # Returns
    ///
    /// This [`DynamicView`] with the child configurator installed.
    #[doc(hidden)]
    pub fn line_numbers(self, line_numbers: bool) -> Self {
        self.inner
            .configurators
            .borrow_mut()
            .push(Box::new(move |child| {
                if let Some(code) = child.downcast_mut::<crate::CodeBlockView>() {
                    *code = code.clone().line_numbers(line_numbers);
                }
            }));
        self
    }

    /// Applies a button action to every child produced by this boundary.
    ///
    /// # Arguments
    ///
    /// * `action` — Callback assigned when the child is a button.
    ///
    /// # Returns
    ///
    /// This [`DynamicView`] with the child configurator installed.
    #[doc(hidden)]
    pub fn on_press(self, action: impl Fn() -> AppControl + 'static) -> Self {
        let action: crate::ButtonAction = Rc::new(action);
        self.inner
            .configurators
            .borrow_mut()
            .push(Box::new(move |child| {
                if let Some(button) = child.downcast_mut::<crate::ButtonView>() {
                    button.on_press = Some(Rc::clone(&action));
                }
            }));
        self
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

    /// Rebuilds an invalidated child and optionally reconciles compatible state.
    fn refresh(&self) {
        if self.inner.current.borrow().is_some() && !self.inner.subscriber.update_if_necessary() {
            return;
        }

        let mut next = match &self.inner.invalidation {
            DynamicInvalidation::Child => self.inner.subscriber.track(|| (self.inner.child)()),
            DynamicInvalidation::Key(key_changed) => {
                let should_refresh = self.inner.subscriber.track(key_changed);
                if self.inner.current.borrow().is_some() && !should_refresh {
                    return;
                }
                untrack(|| (self.inner.child)())
            }
        };
        for configure in self.inner.configurators.borrow().iter() {
            configure(&mut next);
        }
        let mut current = self.inner.current.borrow_mut();

        if self.inner.reconcile_on_refresh
            && let Some(previous) = current.as_ref()
        {
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

    fn __is_layout_transparent(&self) -> bool {
        true
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
    ) -> Option<(usize, u64)> {
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

    fn __scroll_first_overflowing(&mut self, delta: crate::Axes<i16>) -> bool {
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

    fn __scroll_overflowing_at_position(
        &mut self,
        column: u16,
        row: u16,
        delta: crate::Axes<i16>,
    ) -> bool {
        self.with_current_view_mut(|child| {
            child.__scroll_overflowing_at_position(column, row, delta)
        })
    }

    fn __scroll_target_at_position(
        &self,
        column: u16,
        row: u16,
        delta: crate::Axes<i16>,
    ) -> Option<u64> {
        self.with_current_view(|child| child.__scroll_target_at_position(column, row, delta))
    }

    fn __scroll_target_by_paint_order(&mut self, order: u64, delta: crate::Axes<i16>) -> bool {
        self.with_current_view_mut(|child| child.__scroll_target_by_paint_order(order, delta))
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

/// Creates a reactive dynamic child boundary.
///
/// # Arguments
///
/// * `child` — Closure that rebuilds after one of its tracked dependencies changes.
///
/// # Returns
///
/// A [`DynamicView`] retaining compatible child state between reactive refreshes.
pub fn dynamic<V>(child: impl Fn() -> V + 'static) -> DynamicView
where
    V: IntoView,
{
    DynamicView::new(move || child().into_view())
}

/// Creates a reactive child that rebuilds only when its key changes.
///
/// The current child remains mounted while consecutive keys compare equal.
/// A different key replaces the retained child and resets its local view state.
///
/// # Arguments
///
/// * `key` — Callback returning the current invalidation key.
/// * `child` — Callback building the child for a new key.
///
/// # Returns
///
/// A [`DynamicView`] that retains its child across equal keys.
pub fn keyed<K, V>(key: impl Fn() -> K + 'static, child: impl Fn() -> V + 'static) -> DynamicView
where
    K: PartialEq + 'static,
    V: IntoView,
{
    let current_key = RefCell::new(None::<K>);
    DynamicView::new_with_invalidation(
        move || child().into_view(),
        move || {
            let next_key = key();
            let mut current_key = current_key.borrow_mut();
            if current_key.as_ref() == Some(&next_key) {
                false
            } else {
                *current_key = Some(next_key);
                true
            }
        },
        false,
    )
}

#[cfg(test)]
/// Tests for reactive redraw wakeups from dynamic boundaries.
mod tests {
    use std::time::Duration;

    use leptos::prelude::{Get, Owner, RwSignal, Set};
    use tokio::time::timeout;

    use crate::{
        app::{redraw_test_lock, subscribe_redraws},
        text,
    };

    use super::dynamic;

    /// Verifies changing a tracked signal wakes the terminal app loop.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// dynamic(move || text(label.get()))
    /// label.set("Saved")
    /// ```
    ///
    /// # Assertions
    ///
    /// - The initial child evaluation subscribes to the signal.
    /// - Updating the signal delivers a redraw request before timeout.
    #[tokio::test(flavor = "current_thread")]
    async fn tracked_signal_change_requests_redraw() {
        let _redraw_guard = redraw_test_lock().await;
        let owner = Owner::new();
        let (label, view) = owner.with(|| {
            let label = RwSignal::new(String::from("Idle"));
            let view = dynamic(move || text(label.get()));
            view.with_view(|_| ());
            (label, view)
        });
        let mut redraws = subscribe_redraws();

        label.set(String::from("Saved"));

        timeout(Duration::from_secs(1), redraws.changed())
            .await
            .expect("signal invalidation should request a redraw")
            .expect("redraw sender should stay available");
        drop(view);
    }
}
