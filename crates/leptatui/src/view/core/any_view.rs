//! Type erasure for heterogeneous terminal view trees.

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use crossterm::event::{Event, KeyEvent, MouseEvent};
use ratatui::layout::{Rect, Size};

use crate::{
    app::{AppControl, Result},
    component::{FocusedControl, KeyControl, RenderCtx},
    style::{LayoutSize, TuiStyle},
};

use super::{
    contract::View, events, measurement::AvailableSpace, metadata::StyleMetadata,
    render::resolve_style,
};
use crate::MarkdownView;
use crate::component::LayoutPhase;
use crate::view::core::layout::prepare_layout;
use crate::view::media::image::{ImageSource, image_render_area};
use crate::view::reconciliation::reconcile_views;
use crate::view::{
    BlockView, ButtonView, CodeBlockView, ComponentView, DivView, DynamicView, FormView,
    HeadingView, ImageView, InputView, LinkView, ListItemView, ListView, ParagraphView,
    ProgressBarView, TableCellView, TableRowView, TableSectionView, TableView, TextAreaView,
    TextView,
};

/// Owning type-erased view used inside heterogeneous render trees.
pub struct AnyView {
    /// Concrete view node behind the type-erasure boundary.
    inner: Box<dyn View>,
}

impl AnyView {
    /// Returns whether the latest layout pass excluded this subtree.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether this view is in a `display: none` subtree.
    fn is_layout_hidden(&self) -> bool {
        self.inner
            .style_metadata()
            .is_some_and(StyleMetadata::is_layout_hidden)
    }

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
        if self.is_layout_hidden() {
            return Ok(AppControl::Continue);
        }
        self.inner.__dispatch_event(event)
    }

    /// Dispatches a custom key event through the stored subtree.
    #[doc(hidden)]
    pub fn __dispatch_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        if self.is_layout_hidden() {
            return Ok(KeyControl::Pass);
        }
        self.inner.__dispatch_key_event(key)
    }

    /// Emits expired pending input from the stored subtree.
    #[doc(hidden)]
    pub fn __flush_pending_input(&mut self) -> Option<AppControl> {
        if self.is_layout_hidden() {
            return None;
        }
        self.inner.__flush_pending_input()
    }

    /// Returns the number of focusable controls in the stored subtree.
    #[doc(hidden)]
    pub fn __focusable_count(&self) -> usize {
        if self.is_layout_hidden() {
            return 0;
        }
        self.inner.__focusable_count()
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    pub fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        if self.is_layout_hidden() {
            return None;
        }
        self.inner.__focused_index_inner(index)
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    pub fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        if self.is_layout_hidden() {
            return;
        }
        self.inner.__set_focus_by_index_inner(target, index);
    }

    /// Returns the focusable control index under a terminal position.
    ///
    /// # Arguments
    ///
    /// * `column` — Zero-based terminal column to hit test.
    /// * `row` — Zero-based terminal row to hit test.
    /// * `index` — Running flattened focus index to inspect and advance.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the flattened index and global paint ordinal
    /// of the frontmost control under the position.
    #[doc(hidden)]
    pub fn __focusable_index_at_position_inner(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<(usize, u64)> {
        if self.is_layout_hidden() {
            return None;
        }
        self.inner
            .__focusable_index_at_position_inner(column, row, index)
    }

    /// Returns the focused control span in the stored subtree.
    #[doc(hidden)]
    pub fn __focused_button_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        if self.is_layout_hidden() {
            return None;
        }
        self.inner.__focused_control_span(ctx)
    }

    /// Activates the focused button or actionable link in the stored subtree.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the activated control value, if any.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::LinkOpen`] if a focused link cannot be opened.
    #[doc(hidden)]
    pub fn __activate_focused_button(&self) -> Result<Option<AppControl>> {
        if self.is_layout_hidden() {
            return Ok(None);
        }
        self.inner.__activate_focused_button()
    }

    /// Handles a key on the focused editor in the stored subtree.
    #[doc(hidden)]
    pub fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        if self.is_layout_hidden() {
            return None;
        }
        self.inner.__handle_focused_input_key(key)
    }

    /// Returns the focused control in the stored subtree.
    #[doc(hidden)]
    pub fn __focused_control(&self) -> Option<FocusedControl> {
        if self.is_layout_hidden() {
            return None;
        }
        self.inner.__focused_control()
    }

    /// Handles a form-owned key in the stored subtree.
    #[doc(hidden)]
    pub fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        if self.is_layout_hidden() {
            return None;
        }
        self.inner.__handle_form_key(key)
    }

    /// Scrolls the first overflowing container in the stored subtree.
    ///
    /// # Arguments
    ///
    /// * `delta` — Signed horizontal and vertical cell deltas.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether an offset changed.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing(&mut self, delta: crate::Axes<i16>) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__scroll_first_overflowing(delta)
    }

    /// Scrolls the first overflowing container to the top.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__scroll_first_overflowing_to_top()
    }

    /// Scrolls the first overflowing container to the bottom.
    #[doc(hidden)]
    pub fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__scroll_first_overflowing_to_bottom()
    }

    /// Returns whether the stored subtree contains an overflowing container.
    #[doc(hidden)]
    pub fn __has_overflowing_scroll_target(&self) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__has_overflowing_scroll_target()
    }

    /// Handles built-in mouse behavior in the stored subtree.
    ///
    /// # Arguments
    ///
    /// * `mouse` — Crossterm mouse event to handle.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the [`AppControl`] produced by mouse handling.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::LinkOpen`] if clicking a focused link cannot
    /// open its target.
    #[doc(hidden)]
    pub fn __handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<AppControl> {
        if self.is_layout_hidden() {
            return Ok(AppControl::Continue);
        }
        self.inner.__handle_mouse_event(mouse)
    }

    /// Moves focus to the control under a terminal position.
    ///
    /// # Arguments
    ///
    /// * `column` — Zero-based terminal column to hit test.
    /// * `row` — Zero-based terminal row to hit test.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether a focusable control was found.
    #[doc(hidden)]
    pub fn __focus_control_at_position(&mut self, column: u16, row: u16) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__focus_control_at_position(column, row)
    }

    /// Scrolls the innermost overflowing layout under a terminal position.
    ///
    /// # Arguments
    ///
    /// * `column` — Zero-based terminal column to hit test.
    /// * `row` — Zero-based terminal row to hit test.
    /// * `delta` — Signed horizontal and vertical cell deltas.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether a positioned layout consumed the scroll.
    #[doc(hidden)]
    pub fn __scroll_overflowing_at_position(
        &mut self,
        column: u16,
        row: u16,
        delta: crate::Axes<i16>,
    ) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner
            .__scroll_overflowing_at_position(column, row, delta)
    }

    /// Returns the frontmost painted scroll target that can consume a delta.
    ///
    /// # Arguments
    ///
    /// * `column` — Zero-based terminal column to hit test.
    /// * `row` — Zero-based terminal row to hit test.
    /// * `delta` — Signed horizontal and vertical cell deltas.
    ///
    /// # Returns
    ///
    /// An optional `u64` containing the target's global paint ordinal.
    #[doc(hidden)]
    pub fn __scroll_target_at_position(
        &self,
        column: u16,
        row: u16,
        delta: crate::Axes<i16>,
    ) -> Option<u64> {
        if self.is_layout_hidden() {
            return None;
        }
        self.inner.__scroll_target_at_position(column, row, delta)
    }

    /// Scrolls the view whose latest paint ordinal matches one target.
    ///
    /// # Arguments
    ///
    /// * `order` — Global paint ordinal selected during read-only hit testing.
    /// * `delta` — Signed horizontal and vertical cell deltas.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether the selected target changed offsets.
    #[doc(hidden)]
    pub fn __scroll_target_by_paint_order(&mut self, order: u64, delta: crate::Axes<i16>) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__scroll_target_by_paint_order(order, delta)
    }

    /// Stores the pending first key of the `gg` sequence in the stored subtree.
    #[doc(hidden)]
    pub fn __set_scroll_to_top_key_pending(&self, pending: bool) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__set_scroll_to_top_key_pending(pending)
    }

    /// Clears and returns the pending first key of the `gg` sequence.
    #[doc(hidden)]
    pub fn __take_scroll_to_top_key_pending(&self) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__take_scroll_to_top_key_pending()
    }

    /// Returns the focused actionable link target in the stored subtree.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing a clone of the focused actionable target.
    #[doc(hidden)]
    pub fn __focused_link_target(&self) -> Option<crate::LinkTarget> {
        if self.is_layout_hidden() {
            return None;
        }
        self.inner.__focused_link_target()
    }

    /// Requests top-aligned scrolling to the first stored view with `id`.
    ///
    /// # Arguments
    ///
    /// * `id` — Selector identifier of the destination view.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether a matching view accepted the request.
    #[doc(hidden)]
    pub fn __request_scroll_to_id(&mut self, id: &str) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__request_scroll_to_id(id)
    }

    /// Returns whether the stored subtree contains a pending heading anchor.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether anchor scrolling remains pending.
    #[doc(hidden)]
    pub fn __has_scroll_to_anchor_request(&self) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__has_scroll_to_anchor_request()
    }

    /// Moves the first eligible Markdown boundary through cached history.
    ///
    /// # Arguments
    ///
    /// * `back` — Whether to move backward instead of forward.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether a Markdown boundary changed pages.
    #[doc(hidden)]
    pub fn __navigate_markdown_history(&mut self, back: bool) -> bool {
        if self.is_layout_hidden() {
            return false;
        }
        self.inner.__navigate_markdown_history(back)
    }

    /// Clears last-rendered mouse hit areas in the stored subtree.
    #[doc(hidden)]
    pub fn __clear_hit_areas(&self) {
        self.inner.__clear_hit_areas();
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
        compare_type!(DivView);
        compare_type!(FormView);
        compare_type!(ButtonView);
        compare_type!(LinkView);
        compare_type!(InputView);
        compare_type!(TextAreaView);
        compare_type!(ImageView);
        compare_type!(ProgressBarView);
        compare_type!(MarkdownView);
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
    ///
    /// Prior hit areas are cleared before the current root metadata and
    /// concrete node are rendered.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Render context containing the target area and stylesheets.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] after the concrete node renders successfully.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if concrete rendering performs terminal
    /// I/O that fails.
    pub fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let is_layout_root = ctx.layout_phase() == LayoutPhase::Inactive;
        if is_layout_root {
            prepare_layout(self.as_view(), ctx);
        }
        self.inner.__clear_hit_areas();
        if let Some(metadata) = self.inner.style_metadata() {
            if metadata.is_layout_hidden() {
                return Ok(());
            }
            if ctx.honors_layout_geometry() {
                let geometry = metadata
                    .layout_geometry()
                    .expect("styled views should retain geometry before painting");
                ctx.with_layout_geometry(geometry, metadata, |ctx| {
                    ctx.record_metadata_hit_area(metadata);
                    self.as_view().render(ctx)
                })?;
            } else {
                ctx.record_metadata_hit_area(metadata);
                self.as_view().render(ctx)?;
            }
        } else {
            self.as_view().render(ctx)?;
        }
        if is_layout_root {
            super::layout::render_fixed_descendants(self.as_view(), ctx)?;
        }
        Ok(())
    }

    /// Returns the intrinsic size of the stored node.
    ///
    /// # Arguments
    ///
    /// * `known_dimensions` — Exact dimensions supplied by parent layout.
    /// * `available_space` — Soft constraints for unknown dimensions.
    /// * `ctx` — Rendering context containing styles and inherited state.
    ///
    /// # Returns
    ///
    /// A [`LayoutSize`] containing measured terminal-cell width and height.
    pub fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        if self.is_layout_hidden() {
            return LayoutSize::all(0.0);
        }
        self.as_view()
            .measure(known_dimensions, available_space, ctx)
    }

    /// Dispatches an event through the stored subtree.
    pub fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        if self.is_layout_hidden() {
            return Ok(AppControl::Continue);
        }
        self.as_view_mut().handle_event(event)
    }

    /// Dispatches custom and built-in behavior for a key event.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        if self.is_layout_hidden() {
            return Ok(KeyControl::Pass);
        }
        self.as_view_mut().handle_key_event(key)
    }

    /// Handles built-in scrolling, focus, editing, and activation keys.
    #[doc(hidden)]
    pub fn __handle_default_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        if self.is_layout_hidden() {
            return Ok(KeyControl::Pass);
        }
        events::handle_default_view_key_event(self.as_view_mut(), key)
    }

    /// Renders a clipped segment when the stored node is an image.
    ///
    /// # Arguments
    ///
    /// * `source_x` — First source column retained from the full view box.
    /// * `source_y` — First source row retained from the full view box.
    /// * `target_area` — Visible destination rectangle.
    /// * `ctx` — Render context carrying the full view geometry.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing whether the stored node handled image rendering.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if fallback rendering performs terminal
    /// I/O that fails.
    pub(crate) fn render_terminal_image_clipped(
        &self,
        source_x: u16,
        source_y: u16,
        target_area: Rect,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> Result<bool> {
        let Some(image) = self.downcast_ref::<ImageView>() else {
            return Ok(false);
        };

        let style = resolve_style(&image.metadata, ctx);
        let geometry = ctx.layout_geometry();
        let full_image_area = image_render_area(geometry.content_box, style.image_size);
        let source_right = source_x.saturating_add(target_area.width);
        let source_bottom = source_y.saturating_add(target_area.height);
        if source_x >= full_image_area.right()
            || source_y >= full_image_area.bottom()
            || source_right <= full_image_area.x
            || source_bottom <= full_image_area.y
        {
            return Ok(true);
        }

        let visible_source_x = source_x.max(full_image_area.x);
        let visible_source_y = source_y.max(full_image_area.y);
        let image_source_x = visible_source_x.saturating_sub(full_image_area.x);
        let image_source_y = visible_source_y.saturating_sub(full_image_area.y);
        let target_offset_x = visible_source_x.saturating_sub(source_x);
        let target_offset_y = visible_source_y.saturating_sub(source_y);
        let width = full_image_area
            .right()
            .saturating_sub(visible_source_x)
            .min(target_area.width.saturating_sub(target_offset_x));
        let height = full_image_area
            .bottom()
            .saturating_sub(visible_source_y)
            .min(target_area.height.saturating_sub(target_offset_y));
        if width == 0 || height == 0 {
            return Ok(true);
        }

        let ImageSource::Path(path) = &image.source;
        let render_area = Rect {
            x: target_area.x.saturating_add(target_offset_x),
            y: target_area.y.saturating_add(target_offset_y),
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
                image_source_x,
                image_source_y,
            );
        });
        Ok(true)
    }
}
