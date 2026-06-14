//! Component trait contract for terminal rendering.
//!
//! This module defines the render and event-handling interface implemented by
//! root components, child components, and view trees.

use crossterm::event::{Event, KeyEvent};

use super::{key::KeyControl, model::RenderCtx};
use crate::app::{AppControl, Result};

/// Root or child component that can render into a terminal frame.
pub trait Component {
    /// Renders the current component state into the provided context.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context for the component's current area.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if rendering performs terminal I/O
    /// that fails.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()>;

    /// Returns the minimum useful render height for this component.
    #[doc(hidden)]
    fn __min_height(&self, _ctx: &mut RenderCtx<'_, '_>) -> u16 {
        if self.__focusable_count() > 0 { 3 } else { 1 }
    }

    /// Handles a terminal event.
    ///
    /// # Arguments
    ///
    /// * `_event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the app loop should continue.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        if let Event::Key(key) = event {
            return Ok(self.handle_key_event(key)?.into());
        }

        Ok(AppControl::Continue)
    }

    /// Handles a keyboard event with explicit propagation control.
    ///
    /// # Arguments
    ///
    /// * `_key` — Crossterm key event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// A [`KeyControl`] value indicating whether the key was handled.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails.
    fn handle_key_event(&mut self, _key: KeyEvent) -> Result<KeyControl> {
        Ok(KeyControl::Pass)
    }

    /// Dispatches a key event through custom component handlers only.
    #[doc(hidden)]
    fn __dispatch_key_event(&mut self, key: KeyEvent) -> Result<KeyControl> {
        self.handle_key_event(key)
    }

    /// Returns the number of focusable controls inside this component.
    #[doc(hidden)]
    fn __focusable_count(&self) -> usize {
        0
    }

    /// Returns the focused control index while tracking traversal position.
    #[doc(hidden)]
    fn __focused_index_inner(&self, _index: &mut usize) -> Option<usize> {
        None
    }

    /// Sets focus by flattened control index while tracking traversal position.
    #[doc(hidden)]
    fn __set_focus_by_index_inner(&mut self, _target: usize, _index: &mut usize) {}

    /// Returns the focused control's vertical span within this component area.
    #[doc(hidden)]
    fn __focused_button_span(&self, _ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        None
    }

    /// Activates the focused control inside this component, if any.
    #[doc(hidden)]
    fn __activate_focused_button(&self) -> Option<AppControl> {
        None
    }

    /// Scrolls the first overflowing vertical layout inside this component.
    #[doc(hidden)]
    fn __scroll_first_overflowing(&mut self, _delta: i16) -> bool {
        false
    }

    /// Scrolls the first overflowing vertical layout inside this component to the top.
    #[doc(hidden)]
    fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        false
    }

    /// Scrolls the first overflowing vertical layout inside this component to the bottom.
    #[doc(hidden)]
    fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        false
    }

    /// Returns whether this component contains an overflowing scroll target.
    #[doc(hidden)]
    fn __has_overflowing_scroll_target(&self) -> bool {
        false
    }
}
