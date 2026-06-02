use crossterm::event::Event;
use ratatui::Frame;

use crate::{
    component::{Component, RenderCtx},
    context,
};

use super::{AppControl, Result};

/// Runtime adapter consumed by `App`.
pub trait AppRoot {
    /// Renders the current root state into the Ratatui frame.
    ///
    /// # Arguments
    ///
    /// * `frame` — Ratatui frame for the current draw pass.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if rendering through the terminal
    /// backend fails.
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()>;

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
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        Ok(AppControl::Continue)
    }
}

impl<T> AppRoot for T
where
    T: Component,
{
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        context::__with_context_scope(|| {
            let mut ctx = RenderCtx::new(frame);
            Component::render(self, &mut ctx)
        })
    }

    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        Component::handle_event(self, event)
    }
}
