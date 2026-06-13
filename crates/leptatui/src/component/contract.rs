//! Component trait contract for terminal rendering.
//!
//! This module defines the render and event-handling interface implemented by
//! root components, child components, and node trees.

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
}
