//! Terminal app runner.
//!
//! This module owns terminal setup, event polling, root rendering, and cleanup
//! for Leptatui applications. It adapts either an [`AppRoot`] implementation or
//! a [`View`](crate::View) to a managed Ratatui/Crossterm terminal session.
//!
//! # Modules
//!
//! - `control` — App-loop control-flow decisions returned by roots.
//! - `error` — Runtime error and result types.
//! - `event` — Blocking Crossterm event polling helpers.
//! - `render` — Root drawing helpers.
//! - `root` — Root adapter abstraction used by the app runner.
//! - `terminal` — Managed terminal setup and cleanup.
//! - `wakeup` — Async redraw wakeup coordination.

mod control;
mod error;
mod event;
mod render;
mod root;
mod terminal;
mod wakeup;

use std::time::Duration;

use crate::{AnyView, IntoView};

pub use control::AppControl;
pub use error::{Error, Result};
pub use root::AppRoot;
#[cfg(test)]
pub(crate) use wakeup::redraw_test_lock;
pub(crate) use wakeup::{request_redraw, subscribe_redraws};

use event::next_event;
use render::draw_root;
use terminal::TerminalSession;

/// Time between event polls when no input is available.
const DEFAULT_REDRAW_INTERVAL: Duration = Duration::from_millis(16);

/// Runs a root value in a managed terminal session.
#[derive(Debug)]
pub struct App<R> {
    /// Root view or runtime adapter rendered by the app loop.
    root: R,
    /// Polling timeout that also controls idle redraw cadence.
    redraw_interval: Duration,
}

impl App<AnyView> {
    /// Creates an app runner for a root view.
    ///
    /// # Arguments
    ///
    /// * `root` — View-compatible root value to render.
    ///
    /// # Returns
    ///
    /// An [`App`] configured with the default redraw interval.
    pub fn new(root: impl IntoView) -> Self {
        Self {
            root: root.into_view(),
            redraw_interval: DEFAULT_REDRAW_INTERVAL,
        }
    }
}

impl<R> App<R> {
    /// Creates an app from a low-level root adapter.
    ///
    /// # Arguments
    ///
    /// * `root` — Root adapter that owns frame and event integration.
    ///
    /// # Returns
    ///
    /// An [`App`] configured with the default redraw interval.
    pub fn from_root(root: R) -> Self {
        Self {
            root,
            redraw_interval: DEFAULT_REDRAW_INTERVAL,
        }
    }

    /// Overrides the polling timeout that drives periodic redraws.
    ///
    /// # Arguments
    ///
    /// * `redraw_interval` — Non-zero event polling timeout and idle redraw
    ///   cadence. Async redraw requests can wake the runner before this timeout.
    ///
    /// # Returns
    ///
    /// An [`App`] configured with the provided redraw interval.
    ///
    /// # Panics
    ///
    /// Panics if `redraw_interval` is zero.
    pub fn with_redraw_interval(mut self, redraw_interval: Duration) -> Self {
        assert!(
            !redraw_interval.is_zero(),
            "redraw interval must be greater than zero"
        );
        self.redraw_interval = redraw_interval;
        self
    }
}

impl<R> App<R>
where
    R: AppRoot,
{
    /// Runs the app until the root requests exit or a runtime error occurs.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on successful exit and terminal cleanup.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup
    /// fails. Returns [`Error::EventTask`] if the blocking event task fails.
    pub async fn run(mut self) -> Result<()> {
        let mut session = TerminalSession::enter()?;

        let loop_result = self.run_loop(&mut session).await;
        let restore_result = session.restore();

        loop_result?;
        restore_result?;

        Ok(())
    }

    /// Runs the draw and event polling loop for an entered terminal session.
    ///
    /// # Arguments
    ///
    /// * `session` — Active terminal session to draw into and eventually leave.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] when the root requests exit.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if drawing, event polling, or event reading fails.
    /// Returns [`Error::EventTask`] if the blocking event task fails.
    async fn run_loop(&mut self, session: &mut TerminalSession) -> Result<()> {
        let mut redraw_requests = subscribe_redraws();
        let mut should_draw = true;
        let event_poll = next_event(self.redraw_interval);
        tokio::pin!(event_poll);

        loop {
            if should_draw {
                draw_root(
                    &mut self.root,
                    &mut session.terminal,
                    &session.terminal_images,
                )?;
                should_draw = false;
            }

            tokio::select! {
                event = &mut event_poll => {
                    match event? {
                        Some(event) => {
                            if self.root.handle_event(event)? == AppControl::Exit {
                                break;
                            }
                        }
                        None => {
                            if let Some(control) = self.root.__flush_pending_input()
                                && control == AppControl::Exit
                            {
                                break;
                            }
                        }
                    }

                    should_draw = true;
                    event_poll.set(next_event(self.redraw_interval));
                }
                changed = redraw_requests.changed() => {
                    if changed.is_ok() {
                        should_draw = true;
                    }
                }
            }
        }

        Ok(())
    }
}
