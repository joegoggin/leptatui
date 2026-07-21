//! Type erasure for heterogeneous terminal view trees.

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use crossterm::event::{Event, KeyEvent};
use ratatui::layout::{Rect, Size};

use crate::{
    app::{AppControl, Result},
    component::{FocusedControl, KeyControl, RenderCtx},
    style::TuiStyle,
};

use super::{contract::View, events, metadata::StyleMetadata, render::resolve_style};
use crate::view::media::image::{ImageSource, image_render_area};
use crate::view::reconciliation::reconcile_views;
use crate::view::{
    BlockView, ButtonView, CodeBlockView, ComponentView, DynamicView, FormView, HeadingView,
    ImageView, InputView, LayoutView, ListItemView, ListView, ParagraphView, ProgressBarView,
    TableCellView, TableRowView, TableSectionView, TableView, TextAreaView, TextView,
};

/// Owning type-erased view used inside heterogeneous render trees.
pub struct AnyView {
    /// Concrete view node behind the type-erasure boundary.
    inner: Box<dyn View>,
}

impl AnyView {
    /// Erases a concrete view node.
    ///
    /// # Arguments
    ///
    /// * `view` — Concrete view to store.
    ///
    /// # Returns
    ///
    /// An [`AnyView`] owning `view`.
    pub fn new(view: impl View) -> Self {
        Self {
            inner: Box::new(view),
        }
    }

    /// Returns whether the stored node has concrete type `V`.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether `V` is stored.
    pub fn is<V: View>(&self) -> bool {
        self.inner.as_any().is::<V>()
    }

    /// Downcasts the stored node to `V`.
    ///
    /// # Returns
    ///
    /// An optional shared reference to `V`.
    pub fn downcast_ref<V: View>(&self) -> Option<&V> {
        self.inner.as_any().downcast_ref()
    }

    /// Mutably downcasts the stored node to `V`.
    ///
    /// # Returns
    ///
    /// An optional mutable reference to `V`.
    pub fn downcast_mut<V: View>(&mut self) -> Option<&mut V> {
        self.inner.as_any_mut().downcast_mut()
    }

    /// Returns the underlying node contract.
    ///
    /// # Returns
    ///
    /// A shared [`View`] trait object.
    pub fn as_view(&self) -> &dyn View {
        self.inner.as_ref()
    }

    /// Returns the mutable underlying node contract.
    ///
    /// # Returns
    ///
    /// A mutable [`View`] trait object.
    pub fn as_view_mut(&mut self) -> &mut dyn View {
        self.inner.as_mut()
    }

    /// Returns selector metadata for the stored node.
    ///
    /// # Returns
    ///
    /// An optional shared [`StyleMetadata`] reference.
    pub fn style_metadata(&self) -> Option<&StyleMetadata> {
        self.inner.style_metadata()
    }

    /// Returns mutable selector metadata for the stored node.
    ///
    /// # Returns
    ///
    /// An optional mutable [`StyleMetadata`] reference.
    pub fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        self.inner.style_metadata_mut()
    }

    /// Sets an id selector value when the stored node is styleable.
    ///
    /// # Arguments
    ///
    /// * `id` — Id selector value to store.
    ///
    /// # Returns
    ///
    /// This type-erased view after applying `id` when metadata is available.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_id(id);
        }
        self
    }

    /// Sets whitespace-separated class selectors when the stored node is styleable.
    ///
    /// # Arguments
    ///
    /// * `classes` — Whitespace-separated class selector values to store.
    ///
    /// # Returns
    ///
    /// This type-erased view after applying `classes` when metadata is available.
    pub fn with_classes(mut self, classes: impl Into<String>) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_classes(classes);
        }
        self
    }

    /// Sets an inline style override when the stored node is styleable.
    ///
    /// # Arguments
    ///
    /// * `style` — Inline style override to store.
    ///
    /// # Returns
    ///
    /// This type-erased view after applying `style` when metadata is available.
    pub fn with_inline_style(mut self, style: TuiStyle) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_inline_style(style);
        }
        self
    }

    /// Sets focus state when the stored node is styleable.
    ///
    /// # Arguments
    ///
    /// * `focused` — Whether this view should match the focus selector.
    ///
    /// # Returns
    ///
    /// This type-erased view after applying the focus state when possible.
    pub fn with_focus(mut self, focused: bool) -> Self {
        if let Some(metadata) = self.style_metadata_mut() {
            metadata.set_focused(focused);
        }
        self
    }

    /// Returns direct children of the stored node.
    ///
    /// # Returns
    ///
    /// A slice containing direct children.
    pub fn children(&self) -> &[AnyView] {
        self.inner.children()
    }

    /// Returns mutable direct children of the stored node.
    ///
    /// # Returns
    ///
    /// A mutable slice containing direct children.
    pub fn children_mut(&mut self) -> &mut [AnyView] {
        self.inner.children_mut()
    }

    /// Reconciles compatible retained state from a previous tree.
    ///
    /// # Arguments
    ///
    /// * `previous` — Previously rendered type-erased view.
    pub fn reconcile_from(&mut self, previous: &Self) {
        reconcile_views(self.inner.as_mut(), previous.inner.as_ref());
    }

    /// Dispatches a non-default event through the stored subtree.
    #[doc(hidden)]
    pub fn __dispatch_event(&mut self, event: &Event) -> Result<AppControl> {
        self.inner.__dispatch_event(event)
    }

    /// Dispatches a custom key event through the stored subtree.
    #[doc(hidden)]
    pub fn __dispatch_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        self.inner.__dispatch_key_event(key)
    }

    /// Emits expired pending input from the stored subtree.
    #[doc(hidden)]
    pub fn __flush_pending_input(&mut self) -> Option<AppControl> {
        self.inner.__flush_pending_input()
    }

    /// Returns the number of focusable controls in the stored subtree.
    #[doc(hidden)]
    pub fn __focusable_count(&self) -> usize {
        self.inner.__focusable_count()
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    pub fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.inner.__focused_index_inner(index)
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    pub fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        self.inner.__set_focus_by_index_inner(target, index);
    }

    /// Returns the focused control span in the stored subtree.
    #[doc(hidden)]
    pub fn __focused_button_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.inner.__focused_control_span(ctx)
    }

    /// Activates the focused button in the stored subtree.
    #[doc(hidden)]
    pub fn __activate_focused_button(&self) -> Option<AppControl> {
        self.inner.__activate_focused_button()
    }

    /// Handles a key on the focused editor in the stored subtree.
    #[doc(hidden)]
    pub fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.inner.__handle_focused_input_key(key)
    }

    /// Returns the focused control in the stored subtree.
    #[doc(hidden)]
    pub fn __focused_control(&self) -> Option<FocusedControl> {
        self.inner.__focused_control()
    }

    /// Handles a form-owned key in the stored subtree.
    #[doc(hidden)]
    pub fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.inner.__handle_form_key(key)
    }

    /// Scrolls the first overflowing container in the stored subtree.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
        self.inner.__scroll_first_overflowing(delta)
    }

    /// Scrolls the first overflowing container to the top.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        self.inner.__scroll_first_overflowing_to_top()
    }

    /// Scrolls the first overflowing container to the bottom.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        self.inner.__scroll_first_overflowing_to_bottom()
    }

    /// Returns whether the stored subtree contains an overflowing container.
    #[doc(hidden)]
    pub fn __has_overflowing_scroll_target(&self) -> bool {
        self.inner.__has_overflowing_scroll_target()
    }

    /// Stores the pending first key of the `gg` sequence in the stored subtree.
    #[doc(hidden)]
    pub fn __set_scroll_to_top_key_pending(&self, pending: bool) -> bool {
        self.inner.__set_scroll_to_top_key_pending(pending)
    }

    /// Clears and returns the pending first key of the `gg` sequence.
    #[doc(hidden)]
    pub fn __take_scroll_to_top_key_pending(&self) -> bool {
        self.inner.__take_scroll_to_top_key_pending()
    }
}

impl fmt::Debug for AnyView {
    /// Formats a type-erased view using its concrete type name.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnyView")
            .field("type_id", &self.inner.as_any().type_id())
            .field(
                "view_type",
                &self.inner.style_metadata().map(StyleMetadata::view_type),
            )
            .finish()
    }
}

impl Deref for AnyView {
    type Target = dyn View;

    /// Borrows the stored view contract.
    fn deref(&self) -> &Self::Target {
        self.as_view()
    }
}

impl DerefMut for AnyView {
    /// Mutably borrows the stored view contract.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_view_mut()
    }
}

impl PartialEq for AnyView {
    /// Compares built-in type-erased nodes by their concrete values.
    fn eq(&self, other: &Self) -> bool {
        macro_rules! compare_type {
            ($type:ty) => {
                if let Some(left) = self.downcast_ref::<$type>() {
                    return other
                        .downcast_ref::<$type>()
                        .is_some_and(|right| left == right);
                }
            };
        }

        compare_type!(BlockView);
        compare_type!(TextView);
        compare_type!(HeadingView);
        compare_type!(ParagraphView);
        compare_type!(CodeBlockView);
        compare_type!(ListView);
        compare_type!(ListItemView);
        compare_type!(TableView);
        compare_type!(TableSectionView);
        compare_type!(TableRowView);
        compare_type!(TableCellView);
        compare_type!(LayoutView);
        compare_type!(FormView);
        compare_type!(ButtonView);
        compare_type!(InputView);
        compare_type!(TextAreaView);
        compare_type!(ImageView);
        compare_type!(ProgressBarView);
        compare_type!(DynamicView);
        compare_type!(ComponentView);

        false
    }
}

impl<V> PartialEq<V> for AnyView
where
    V: View + PartialEq,
{
    /// Compares a type-erased node with a concrete view of the same type.
    fn eq(&self, other: &V) -> bool {
        self.downcast_ref::<V>().is_some_and(|view| view == other)
    }
}

impl AnyView {
    /// Renders the stored concrete node.
    pub fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        self.as_view().render(ctx)
    }

    /// Returns the minimum useful height of the stored subtree.
    #[doc(hidden)]
    pub fn __min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        self.as_view().min_height(ctx)
    }

    /// Dispatches an event through the stored subtree.
    pub fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        self.as_view_mut().handle_event(event)
    }

    /// Dispatches custom and built-in behavior for a key event.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        self.as_view_mut().handle_key_event(key)
    }

    /// Handles built-in scrolling, focus, editing, and activation keys.
    #[doc(hidden)]
    pub fn __handle_default_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        events::handle_default_view_key_event(self.as_view_mut(), key)
    }

    /// Renders a clipped segment when the stored node is an image.
    pub(crate) fn render_terminal_image_clipped(
        &self,
        source_y: u16,
        target_area: Rect,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> Result<bool> {
        let Some(image) = self.downcast_ref::<ImageView>() else {
            return Ok(false);
        };

        let style = resolve_style(&image.metadata, ctx);
        let full_image_area = image_render_area(ctx.area(), style.image_size);
        if source_y >= full_image_area.height {
            return Ok(true);
        }

        let width = full_image_area.width.min(target_area.width);
        let height = full_image_area
            .height
            .saturating_sub(source_y)
            .min(target_area.height);
        if width == 0 || height == 0 {
            return Ok(true);
        }

        let ImageSource::Path(path) = &image.source;
        let render_area = Rect {
            x: target_area
                .x
                .saturating_add(full_image_area.x.saturating_sub(ctx.area().x)),
            y: target_area.y,
            width,
            height,
        };
        let full_size = Size::new(full_image_area.width, full_image_area.height);
        ctx.with_area(render_area, |ctx| {
            ctx.render_terminal_image_path_clipped(
                path,
                image.alt.as_deref(),
                style.to_ratatui_style(),
                full_size,
                source_y,
            );
        });
        Ok(true)
    }
}
