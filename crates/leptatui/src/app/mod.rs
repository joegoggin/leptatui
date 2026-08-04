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
//! - `handle` — Component-facing terminal suspension requests.
//! - `render` — Root drawing helpers.
//! - `root` — Root adapter abstraction used by the app runner.
//! - `terminal` — Managed terminal setup and cleanup.
//! - `wakeup` — Async redraw wakeup coordination.

mod control;
mod error;
mod error_screen;
mod event;
mod handle;
mod render;
mod root;
mod terminal;
mod wakeup;

use std::time::Duration;

use futures_util::FutureExt;

use crate::executor::init_tokio_executor;
use crate::{AnyView, IntoView, View};

pub use control::AppControl;
pub use error::{Error, Result};
pub(crate) use error_screen::ErrorScreenRegistry;
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
    /// Active standalone error screen scoped to this runner.
    error_screens: ErrorScreenRegistry,
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
            error_screens: ErrorScreenRegistry::new(),
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
            error_screens: ErrorScreenRegistry::new(),
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
    /// Dispatches an event to the active error screen or ordinary app root.
    ///
    /// # Arguments
    ///
    /// * `event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the app should exit.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if event handling performs terminal I/O that
    /// fails. Returns [`Error::LinkOpen`] if an activated link cannot open.
    fn handle_active_event(&mut self, event: crossterm::event::Event) -> Result<AppControl> {
        if let Some(mut screen) = self.error_screens.active() {
            View::handle_event(&mut screen, event)
        } else {
            self.root.handle_event(event)
        }
    }

    /// Flushes pending input from the active error screen or ordinary root.
    ///
    /// # Returns
    ///
    /// An optional [`AppControl`] emitted by pending input handling.
    fn flush_active_input(&mut self) -> Option<AppControl> {
        if let Some(mut screen) = self.error_screens.active() {
            View::__flush_pending_input(&mut screen)
        } else {
            self.root.__flush_pending_input()
        }
    }

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
    pub async fn run(mut self) -> Result<()> {
        init_tokio_executor();
        tokio::task::LocalSet::new()
            .run_until(async move { self.run_managed().await })
            .await
    }

    /// Runs the managed terminal session inside the local task set.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] after terminal cleanup.
    ///
    /// # Errors
    ///
    /// Returns a runtime [`Error`] if terminal setup, rendering, input,
    /// cleanup, event polling, or link activation fails.
    async fn run_managed(&mut self) -> Result<()> {
        let mut session = TerminalSession::enter()?;
        let panic_hook = session.install_panic_hook();

        let loop_result = std::panic::AssertUnwindSafe(self.run_loop(&mut session))
            .catch_unwind()
            .await;
        let restore_result = session.restore();
        drop(panic_hook);

        let loop_result = match loop_result {
            Ok(result) => result,
            Err(panic) => {
                let _ = restore_result;
                std::panic::resume_unwind(panic);
            }
        };

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
    /// Returns [`Error::LinkOpen`] if an activated link cannot be opened.
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
                    &self.error_screens,
                )?;
                should_draw = false;
            }

            tokio::select! {
                events = &mut event_poll => {
                    let events = events?;
                    if events.is_empty() {
                        if let Some(control) = self.flush_active_input()
                            && control == AppControl::Exit
                        {
                            break;
                        }
                    } else {
                        for event in events {
                            if self.handle_active_event(event)? == AppControl::Exit {
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
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Frame;

    use crate::{button, text, view::ComponentView};

    use super::*;

    /// App root that records every event dispatched to the ordinary screen.
    struct EventCountingRoot {
        /// Shared number of events received by this root.
        event_count: Rc<Cell<usize>>,
    }

    impl AppRoot for EventCountingRoot {
        /// Leaves the test frame unchanged.
        ///
        /// # Arguments
        ///
        /// * `_frame` — Unused Ratatui frame supplied by the app root contract.
        ///
        /// # Returns
        ///
        /// An empty [`Result`] for the render-only test root.
        fn render(&mut self, _frame: &mut Frame<'_>) -> Result<()> {
            Ok(())
        }

        /// Records one event dispatched to the ordinary app root.
        ///
        /// # Arguments
        ///
        /// * `_event` — Event counted by the test root.
        ///
        /// # Returns
        ///
        /// An [`AppControl::Continue`] value after recording the event.
        fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
            self.event_count
                .set(self.event_count.get().saturating_add(1));
            Ok(AppControl::Continue)
        }
    }

    /// Creates a plain key-press event for app dispatch tests.
    ///
    /// # Arguments
    ///
    /// * `code` — Key code placed in the event.
    ///
    /// # Returns
    ///
    /// An [`Event`] containing the requested key press.
    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

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

    /// Verifies an active error screen exclusively owns app event dispatch.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// ordinary root + active focused Exit button
    /// Enter -> error screen
    /// dismiss
    /// Enter -> ordinary root
    /// ```
    ///
    /// # Assertions
    ///
    /// - Enter activates the error-screen button and requests app exit.
    /// - The mounted ordinary root receives no event while the screen is active.
    /// - The same ordinary root receives events again after dismissal.
    #[test]
    fn active_error_screen_exclusively_handles_events_and_preserves_root() {
        let event_count = Rc::new(Cell::new(0));
        let mut app = App::from_root(EventCountingRoot {
            event_count: event_count.clone(),
        });
        let screen = ComponentView::new(
            button("Quit")
                .on_press(|| AppControl::Exit)
                .with_focus(true),
        );
        app.error_screens.register(&screen);

        assert_eq!(
            app.handle_active_event(key(KeyCode::Enter))
                .expect("error-screen event dispatch should succeed"),
            AppControl::Exit,
        );
        assert_eq!(event_count.get(), 0);

        app.error_screens.dismiss();
        assert_eq!(
            app.handle_active_event(key(KeyCode::Enter))
                .expect("ordinary root event dispatch should succeed"),
            AppControl::Continue,
        );
        assert_eq!(event_count.get(), 1);
    }
}
