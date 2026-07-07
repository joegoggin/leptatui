//! App root adapter contract.
//!
//! This module defines the root-level rendering interface and adapts
//! [`Component`] values into app roots.

use crossterm::event::Event;
use ratatui::Frame;

use crate::{
    component::{Component, FocusedControl, RenderCtx},
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

    /// Returns metadata for the focused built-in control inside this root.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<FocusedControl> {
        None
    }
}

impl<T> AppRoot for T
where
    T: Component,
{
    /// Renders a component root inside a fresh Leptatui context scope.
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
    /// Returns [`crate::app::Error::Io`] if component rendering performs terminal
    /// I/O that fails.
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        context::hooks::__with_context_scope(|| {
            let mut ctx = RenderCtx::new(frame);
            Component::render(self, &mut ctx)
        })
    }

    /// Forwards a terminal event to the component root.
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
    /// Returns [`crate::app::Error::Io`] if event handling performs terminal
    /// I/O that fails.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        Component::handle_event(self, event)
    }

    /// Forwards focused-control metadata from component roots.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<FocusedControl> {
        Component::__focused_control(self)
    }
}
