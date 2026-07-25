//! Object-safe behavior shared by every terminal view.

use std::any::Any;

use crossterm::event::{Event, KeyEvent, MouseEvent};

use crate::{
    app::{AppControl, Result},
    component::{FocusedControl, KeyControl, RenderCtx},
    style::LayoutSize,
    view::LinkTarget,
};

use super::{
    any_view::AnyView,
    measurement::{AvailableSpace, cells_to_u16, measure_default},
    metadata::StyleMetadata,
};

/// Runtime behavior implemented by every terminal view node.
///
/// Only [`render`](Self::render), [`as_any`](Self::as_any), and
/// [`as_any_mut`](Self::as_any_mut) are required for a render-only leaf. The
/// remaining methods provide defaults so custom nodes opt into styling,
/// containers, interaction, and reconciliation independently.
pub trait View: Any {
    /// Renders this node into its current area.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context containing the target area.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if rendering performs terminal I/O that fails.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()>;

    /// Returns this node's intrinsic terminal-cell size.
    ///
    /// # Arguments
    ///
    /// * `known_dimensions` — Exact dimensions supplied by parent layout.
    /// * `available_space` — Soft constraints for unknown dimensions.
    /// * `_ctx` — Rendering context containing styles and inherited state.
    ///
    /// # Returns
    ///
    /// A [`LayoutSize`] containing measured terminal-cell width and height.
    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        _ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        measure_default(known_dimensions, available_space)
    }

    /// Handles a terminal event using custom and built-in view behavior.
    ///
    /// # Arguments
    ///
    /// * `event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the app loop should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if custom event handling performs terminal
    /// I/O that fails. Returns [`crate::Error::LinkOpen`] if an activated link
    /// cannot be opened.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        super::events::handle_view_event(self, event)
    }

    /// Handles a key using custom handlers followed by built-in view behavior.
    ///
    /// # Arguments
    ///
    /// * `key` — Crossterm key event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// A [`KeyControl`] value indicating whether the key was handled.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if custom key handling performs terminal
    /// I/O that fails. Returns [`crate::Error::LinkOpen`] if an activated link
    /// cannot be opened.
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        super::events::handle_view_key_event(self, key)
    }

    /// Returns the minimum useful height for the legacy renderer.
    #[doc(hidden)]
    fn __min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        let area = ctx.area();
        let measured = self.measure(
            LayoutSize::new(Some(f32::from(area.width)), None),
            LayoutSize::new(
                AvailableSpace::Definite(f32::from(area.width)),
                AvailableSpace::Definite(f32::from(area.height)),
            ),
            ctx,
        );
        cells_to_u16(measured.height)
    }

    /// Returns selector metadata when this node participates in styling.
    ///
    /// # Returns
    ///
    /// An optional shared [`StyleMetadata`] reference.
    fn style_metadata(&self) -> Option<&StyleMetadata> {
        None
    }

    /// Returns mutable selector metadata when this node participates in styling.
    ///
    /// # Returns
    ///
    /// An optional mutable [`StyleMetadata`] reference.
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        None
    }

    /// Returns direct children in render order.
    ///
    /// # Returns
    ///
    /// A slice containing direct type-erased children.
    fn children(&self) -> &[AnyView] {
        &[]
    }

    /// Returns mutable direct children in render order.
    ///
    /// # Returns
    ///
    /// A mutable slice containing direct type-erased children.
    fn children_mut(&mut self) -> &mut [AnyView] {
        &mut []
    }

    /// Visits children used to construct the transient layout tree.
    ///
    /// The default exposes ordinary retained children. Structural boundaries
    /// override this hook to expose their materialized child without creating
    /// an additional layout box.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Render context carrying stylesheet and component scopes.
    /// * `visitor` — Callback invoked for each logical layout child.
    #[doc(hidden)]
    fn __visit_layout_children(
        &self,
        ctx: &mut RenderCtx<'_, '_>,
        visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>),
    ) {
        for child in self.children() {
            visitor(child, ctx);
        }
    }

    /// Returns whether this view contributes children without generating a box.
    ///
    /// # Returns
    ///
    /// `true` when the view is a layout-transparent structural boundary.
    #[doc(hidden)]
    fn __is_layout_transparent(&self) -> bool {
        false
    }

    /// Returns whether retained children participate in computed layout.
    ///
    /// # Returns
    ///
    /// `true` when the view's retained children generate layout boxes.
    #[doc(hidden)]
    fn __uses_computed_child_layout(&self) -> bool {
        false
    }

    /// Returns this node as [`Any`] for concrete-type inspection.
    ///
    /// # Returns
    ///
    /// A shared [`Any`] trait object.
    fn as_any(&self) -> &dyn Any;

    /// Returns this node as mutable [`Any`] for concrete-type inspection.
    ///
    /// # Returns
    ///
    /// A mutable [`Any`] trait object.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Reconciles node-specific retained state from a compatible previous node.
    ///
    /// # Arguments
    ///
    /// * `_previous` — Previous node with the same concrete Rust type.
    fn reconcile(&mut self, _previous: &dyn View) {}

    /// Returns whether retained state may be copied from a previous node.
    ///
    /// The default requires identical concrete Rust types. Views that combine
    /// multiple semantic variants in one struct or represent deferred
    /// boundaries should additionally compare their variant or identity.
    ///
    /// # Arguments
    ///
    /// * `previous` — Previous node considered as reconciliation input.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether reconciliation may proceed.
    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        self.as_any().type_id() == previous.as_any().type_id()
    }

    /// Handles an application-defined non-key event for this node.
    ///
    /// The default subtree dispatcher visits children before invoking this
    /// hook. Override it when a custom view needs mouse, focus, paste, resize,
    /// or other non-key event behavior.
    ///
    /// # Arguments
    ///
    /// * `_event` — Event dispatched to this node.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the application should exit.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if custom event handling performs terminal
    /// I/O that fails.
    fn on_event(&mut self, _event: &Event) -> Result<AppControl> {
        Ok(AppControl::Continue)
    }

    /// Handles an application-defined key event for this node.
    ///
    /// The default subtree dispatcher visits children before invoking this
    /// hook. Built-in focus, editing, activation, and scrolling behavior runs
    /// afterward when this hook returns [`KeyControl::Pass`].
    ///
    /// # Arguments
    ///
    /// * `_key` — Key event dispatched to this node.
    ///
    /// # Returns
    ///
    /// A [`KeyControl`] value indicating whether propagation should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if custom event handling performs terminal
    /// I/O that fails.
    fn on_key_event(&mut self, _key: KeyEvent) -> Result<KeyControl> {
        Ok(KeyControl::Pass)
    }

    /// Dispatches a non-default event through this subtree.
    #[doc(hidden)]
    fn __dispatch_event(&mut self, event: &Event) -> Result<AppControl> {
        for child in self.children_mut() {
            let control = child.__dispatch_event(event)?;
            if control == AppControl::Exit {
                return Ok(control);
            }
        }
        self.on_event(event)
    }

    /// Dispatches a custom key event through this subtree.
    #[doc(hidden)]
    fn __dispatch_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        for child in self.children_mut() {
            let control = child.__dispatch_key_event(key)?;
            if control != KeyControl::Pass {
                return Ok(control);
            }
        }
        self.on_key_event(key)
    }

    /// Emits an expired pending input sequence in this subtree.
    #[doc(hidden)]
    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        self.children_mut()
            .iter_mut()
            .find_map(AnyView::__flush_pending_input)
    }

    /// Returns the number of focusable controls in this subtree.
    #[doc(hidden)]
    fn __focusable_count(&self) -> usize {
        self.children().iter().map(AnyView::__focusable_count).sum()
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.children()
            .iter()
            .find_map(|child| child.__focused_index_inner(index))
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        for child in self.children_mut() {
            child.__set_focus_by_index_inner(target, index);
        }
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
    /// An [`Option`] containing the flattened index under the position.
    #[doc(hidden)]
    fn __focusable_index_at_position_inner(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<usize> {
        self.children()
            .iter()
            .find_map(|child| child.__focusable_index_at_position_inner(column, row, index))
    }

    /// Returns the focused control span inside this node.
    #[doc(hidden)]
    fn __focused_control_span(&self, _ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        None
    }

    /// Activates the focused button or actionable link in this subtree.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the activated control value, if any.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::LinkOpen`] if a focused link cannot be opened.
    #[doc(hidden)]
    fn __activate_focused_button(&self) -> Result<Option<AppControl>> {
        for child in self.children() {
            if let Some(control) = child.__activate_focused_button()? {
                return Ok(Some(control));
            }
        }
        Ok(None)
    }

    /// Handles a key on the focused editor in this subtree.
    #[doc(hidden)]
    fn __handle_focused_input_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.children_mut()
            .iter_mut()
            .find_map(|child| child.__handle_focused_input_key(key))
    }

    /// Returns the focused control kind in this subtree.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<FocusedControl> {
        self.children().iter().find_map(AnyView::__focused_control)
    }

    /// Handles form-owned keys in this subtree.
    #[doc(hidden)]
    fn __handle_form_key(&mut self, key: KeyEvent) -> Option<KeyControl> {
        self.children_mut()
            .iter_mut()
            .find_map(|child| child.__handle_form_key(key))
    }

    /// Scrolls the first overflowing container in this subtree.
    ///
    /// # Arguments
    ///
    /// * `delta` — Signed horizontal and vertical cell deltas.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether an offset changed.
    #[doc(hidden)]
    fn __scroll_first_overflowing(&mut self, delta: crate::Axes<i16>) -> bool {
        self.children_mut()
            .iter_mut()
            .any(|child| child.__scroll_first_overflowing(delta))
    }

    /// Scrolls the first overflowing container to the top.
    #[doc(hidden)]
    fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        self.children_mut()
            .iter_mut()
            .any(AnyView::__scroll_first_overflowing_to_top)
    }

    /// Scrolls the first overflowing container to the bottom.
    #[doc(hidden)]
    fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        self.children_mut()
            .iter_mut()
            .any(AnyView::__scroll_first_overflowing_to_bottom)
    }

    /// Returns whether this subtree contains an overflowing container.
    #[doc(hidden)]
    fn __has_overflowing_scroll_target(&self) -> bool {
        self.children()
            .iter()
            .any(AnyView::__has_overflowing_scroll_target)
    }

    /// Handles built-in mouse focus, activation, and scrolling behavior.
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
    fn __handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<AppControl> {
        super::events::handle_default_view_mouse_event(self, mouse)
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
    fn __focus_control_at_position(&mut self, column: u16, row: u16) -> bool {
        let mut index = 0;
        let Some(target) = self.__focusable_index_at_position_inner(column, row, &mut index) else {
            return false;
        };
        let mut index = 0;
        self.__set_focus_by_index_inner(target, &mut index);
        true
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
    fn __scroll_overflowing_at_position(
        &mut self,
        column: u16,
        row: u16,
        delta: crate::Axes<i16>,
    ) -> bool {
        self.children_mut()
            .iter_mut()
            .any(|child| child.__scroll_overflowing_at_position(column, row, delta))
    }

    /// Stores the pending first key of the `gg` sequence in this subtree.
    #[doc(hidden)]
    fn __set_scroll_to_top_key_pending(&self, pending: bool) -> bool {
        if let Some(metadata) = self.style_metadata() {
            metadata.set_scroll_to_top_key_pending(pending);
            return true;
        }

        self.children()
            .iter()
            .any(|child| child.__set_scroll_to_top_key_pending(pending))
    }

    /// Clears and returns the pending first key of the `gg` sequence.
    #[doc(hidden)]
    fn __take_scroll_to_top_key_pending(&self) -> bool {
        if let Some(metadata) = self.style_metadata() {
            return metadata.take_scroll_to_top_key_pending();
        }

        self.children()
            .iter()
            .any(AnyView::__take_scroll_to_top_key_pending)
    }

    /// Returns the focused actionable link target in this subtree.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing a clone of the focused actionable target.
    #[doc(hidden)]
    fn __focused_link_target(&self) -> Option<LinkTarget> {
        self.children()
            .iter()
            .find_map(AnyView::__focused_link_target)
    }

    /// Requests top-aligned scrolling to the first view with `id`.
    ///
    /// # Arguments
    ///
    /// * `id` — Selector identifier of the destination view.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether a matching view accepted the request.
    #[doc(hidden)]
    fn __request_scroll_to_id(&mut self, id: &str) -> bool {
        if let Some(metadata) = self.style_metadata_mut()
            && metadata.id() == Some(id)
        {
            metadata.request_scroll_to_anchor();
            return true;
        }

        self.children_mut()
            .iter_mut()
            .any(|child| child.__request_scroll_to_id(id))
    }

    /// Returns whether this subtree contains a pending heading anchor.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether anchor scrolling remains pending.
    #[doc(hidden)]
    fn __has_scroll_to_anchor_request(&self) -> bool {
        self.style_metadata()
            .is_some_and(StyleMetadata::scroll_to_anchor_requested)
            || self
                .children()
                .iter()
                .any(AnyView::__has_scroll_to_anchor_request)
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
    fn __navigate_markdown_history(&mut self, back: bool) -> bool {
        self.children_mut()
            .iter_mut()
            .any(|child| child.__navigate_markdown_history(back))
    }

    /// Clears last-rendered mouse hit areas throughout this subtree.
    #[doc(hidden)]
    fn __clear_hit_areas(&self) {
        if let Some(metadata) = self.style_metadata() {
            metadata.clear_hit_areas();
        }
        for child in self.children() {
            child.__clear_hit_areas();
        }
    }
}
