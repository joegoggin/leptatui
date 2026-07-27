//! Interaction forwarding across the type-erasure boundary.

use crossterm::event::{Event, KeyEvent, MouseEvent};

use super::AnyView;
use crate::{
    app::{AppControl, Result},
    component::{FocusedControl, KeyControl, RenderCtx},
    view::core::events,
};

impl AnyView {
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

    /// Dispatches an event through the stored subtree.
    ///
    /// # Arguments
    ///
    /// * `event` — Event to dispatch through the stored view.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the resulting application control.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] if the stored view cannot handle the event.
    pub fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        if self.is_layout_hidden() {
            return Ok(AppControl::Continue);
        }
        self.as_view_mut().handle_event(event)
    }

    /// Dispatches custom and built-in behavior for a key event.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to dispatch through the stored view.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the resulting key control.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] if the stored view cannot handle the key event.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        if self.is_layout_hidden() {
            return Ok(KeyControl::Pass);
        }
        self.as_view_mut().handle_key_event(key)
    }

    /// Handles built-in scrolling, focus, editing, and activation keys.
    ///
    /// # Arguments
    ///
    /// * `key` — Key event to process using default view behavior.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the resulting key control.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error`] if default key handling fails.
    #[doc(hidden)]
    pub fn __handle_default_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        if self.is_layout_hidden() {
            return Ok(KeyControl::Pass);
        }
        events::handle_default_view_key_event(self.as_view_mut(), key)
    }
}
