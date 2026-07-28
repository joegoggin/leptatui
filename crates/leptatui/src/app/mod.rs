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
//! - `handle` — Component-facing terminal suspension and exit-error requests.
//! - `render` — Root drawing helpers.
//! - `root` — Root adapter abstraction used by the app runner.
//! - `terminal` — Managed terminal setup and cleanup.
//! - `wakeup` — Async redraw wakeup coordination.

mod control;
mod error;
mod event;
mod handle;
mod render;
mod root;
mod terminal;
mod wakeup;

use std::time::Duration;

use crate::{AnyView, IntoView};

pub use control::AppControl;
pub use error::{Error, Result};
pub use handle::{AppHandle, use_app_handle};
pub use root::AppRoot;
#[cfg(test)]
pub(crate) use wakeup::redraw_test_lock;
pub(crate) use wakeup::{request_redraw, subscribe_redraws};

use event::next_events;
use render::draw_root;
use terminal::TerminalSession;

/// Time between event polls when no input is available.
const DEFAULT_REDRAW_INTERVAL: Duration = Duration::from_millis(16);

/// Runs a root value in a managed terminal session.
#[derive(Debug)]
pub struct App<R> {
    /// Root view or runtime adapter rendered by the app loop.
    root: R,
    /// Component-facing runtime handle scoped to this runner.
    handle: AppHandle,
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
            handle: AppHandle::new(),
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
            handle: AppHandle::new(),
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
    /// Returns [`Error::LinkOpen`] if an activated link cannot be opened.
    /// Returns [`Error::Application`] if a component records an exit error
    /// before requesting shutdown.
    pub async fn run(mut self) -> Result<()> {
        let mut session = TerminalSession::enter()?;

        let loop_result = self.run_loop(&mut session).await;
        let restore_result = session.restore();

        loop_result?;
        restore_result?;

        if let Some(error) = self.handle.take_exit_error() {
            return Err(error);
        }

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
    /// Returns [`Error::LinkOpen`] if an activated link cannot be opened.
    /// Returns [`Error::Application`] after an exit request when a component
    /// recorded an application failure.
    async fn run_loop(&mut self, session: &mut TerminalSession) -> Result<()> {
        let mut redraw_requests = subscribe_redraws();
        let mut should_draw = true;
        let event_poll = next_events(self.redraw_interval);
        tokio::pin!(event_poll);

        'app: loop {
            if should_draw {
                draw_root(
                    &mut self.root,
                    &mut session.terminal,
                    &session.terminal_images,
                    &self.handle,
                )?;
                should_draw = false;
            }

            tokio::select! {
                events = &mut event_poll => {
                    let events = events?;
                    if events.is_empty() {
                        if let Some(control) = self.root.__flush_pending_input()
                            && control == AppControl::Exit
                        {
                            break;
                        }
                    } else {
                        for event in events {
                            if self.root.handle_event(event)? == AppControl::Exit {
                                break 'app;
                            }

                            if self.run_suspended_tasks(session)? {
                                break;
                            }
                        }
                    }

                    should_draw = true;
                    event_poll.set(next_events(self.redraw_interval));
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

    /// Executes queued component tasks outside managed terminal modes.
    ///
    /// # Arguments
    ///
    /// * `session` — Active terminal session to restore and re-enter.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether any tasks were executed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if terminal restoration or re-entry fails.
    fn run_suspended_tasks(&self, session: &mut TerminalSession) -> Result<bool> {
        let mut next_session = None;
        let executed = self.run_suspended_tasks_with(
            || session.restore().map_err(Error::from),
            || {
                next_session = Some(TerminalSession::enter()?);
                Ok(())
            },
        )?;

        if let Some(next_session) = next_session {
            *session = next_session;
        }

        Ok(executed)
    }

    /// Executes queued tasks between caller-supplied terminal transitions.
    ///
    /// # Arguments
    ///
    /// * `restore` — Operation that releases managed terminal modes.
    /// * `reenter` — Operation that re-enters managed terminal modes.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether any tasks were executed.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] produced by either terminal transition.
    fn run_suspended_tasks_with(
        &self,
        restore: impl FnOnce() -> Result<()>,
        reenter: impl FnOnce() -> Result<()>,
    ) -> Result<bool> {
        let tasks = self.handle.take_suspended_tasks();
        if tasks.is_empty() {
            return Ok(false);
        }

        restore()?;
        for task in tasks {
            task();
        }
        reenter()?;

        Ok(true)
    }
}

#[cfg(test)]
/// Unit tests for component-requested runtime transitions.
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::text;

    use super::*;

    /// Verifies terminal suspension surrounds queued component work.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// restore terminal
    /// run task 1
    /// run task 2
    /// re-enter terminal
    /// ```
    ///
    /// # Assertions
    ///
    /// - The helper reports that work executed.
    /// - Restoration occurs before both FIFO tasks.
    /// - Terminal re-entry occurs after every task.
    #[test]
    fn suspended_tasks_run_between_restore_and_reentry() {
        let app = App::new(text("test"));
        let order = Rc::new(RefCell::new(Vec::new()));

        for step in ["task-1", "task-2"] {
            let order = order.clone();
            app.handle
                .suspend_terminal(move || order.borrow_mut().push(step));
        }

        let restore_order = order.clone();
        let reenter_order = order.clone();
        let executed = app
            .run_suspended_tasks_with(
                move || {
                    restore_order.borrow_mut().push("restore");
                    Ok(())
                },
                move || {
                    reenter_order.borrow_mut().push("reenter");
                    Ok(())
                },
            )
            .expect("terminal transitions should succeed");

        assert!(executed);
        assert_eq!(
            *order.borrow(),
            vec!["restore", "task-1", "task-2", "reenter"]
        );
    }

    /// Verifies empty suspension queues do not transition the terminal.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// AppHandle with no suspended tasks
    /// ```
    ///
    /// # Assertions
    ///
    /// - The helper reports that no work executed.
    /// - Neither terminal transition callback runs.
    #[test]
    fn empty_suspension_queue_skips_terminal_transitions() {
        let app = App::new(text("test"));
        let transitions = Rc::new(RefCell::new(0));
        let restore_transitions = transitions.clone();
        let reenter_transitions = transitions.clone();

        let executed = app
            .run_suspended_tasks_with(
                move || {
                    *restore_transitions.borrow_mut() += 1;
                    Ok(())
                },
                move || {
                    *reenter_transitions.borrow_mut() += 1;
                    Ok(())
                },
            )
            .expect("an empty queue should succeed");

        assert!(!executed);
        assert_eq!(*transitions.borrow(), 0);
    }
}
