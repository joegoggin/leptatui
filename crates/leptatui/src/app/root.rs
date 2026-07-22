//! App root adapter contract.
//!
//! This module defines the root-level rendering interface and adapts [`View`]
//! values into app roots.

use crossterm::event::Event;
use ratatui::Frame;

use crate::{
    AnyView, View,
    component::{FocusedControl, RenderCtx},
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
    /// I/O that fails. Returns [`crate::app::Error::LinkOpen`] if an activated
    /// link cannot be opened.
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        Ok(AppControl::Continue)
    }

    /// Emits any expired pending input inside this root.
    #[doc(hidden)]
    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        None
    }

    /// Returns metadata for the focused built-in control inside this root.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<FocusedControl> {
        None
    }
}

impl AppRoot for AnyView {
    /// Renders a view root inside a fresh Leptatui context scope.
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
    /// Returns [`crate::app::Error::Io`] if view rendering performs terminal
    /// I/O that fails.
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        context::hooks::__with_context_scope(|| {
            let mut ctx = RenderCtx::new(frame);
            AnyView::render(self, &mut ctx)
        })
    }

    /// Forwards a terminal event to the view root.
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
    /// I/O that fails. Returns [`crate::app::Error::LinkOpen`] if an activated
    /// link cannot be opened.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        AnyView::handle_event(self, event)
    }

    /// Forwards pending input flushing into the view root.
    #[doc(hidden)]
    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        AnyView::__flush_pending_input(self)
    }

    /// Forwards focused-control metadata from the view root.
    #[doc(hidden)]
    fn __focused_control(&self) -> Option<FocusedControl> {
        AnyView::__focused_control(self)
    }
}

impl<V> AppRoot for V
where
    V: View,
{
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        context::hooks::__with_context_scope(|| {
            let mut ctx = RenderCtx::new(frame);
            View::render(self, &mut ctx)
        })
    }

    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        View::handle_event(self, event)
    }

    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        View::__flush_pending_input(self)
    }

    fn __focused_control(&self) -> Option<FocusedControl> {
        View::__focused_control(self)
    }
}
