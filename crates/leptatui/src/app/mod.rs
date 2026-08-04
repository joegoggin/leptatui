//! Terminal app runner.
//!
//! This module owns terminal setup, event polling, root rendering, and cleanup
//! for Leptatui applications. It adapts either an [`AppRoot`] implementation or
//! a [`View`] to a managed Ratatui/Crossterm terminal session.
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
use crate::{AnyView, IntoView, View, view::VisitedLinkRegistry};

pub use control::AppControl;
pub use error::{Error, Result};
pub(crate) use error_screen::ErrorScreenRegistry;
pub use handle::{AppHandle, use_app_handle};
pub use root::AppRoot;
pub(crate) use root::{EventOutcome, LayoutMode};
#[cfg(test)]
pub(crate) use wakeup::redraw_test_lock;
pub(crate) use wakeup::{request_redraw, subscribe_redraws};

use event::next_events;
use render::draw_root;
use terminal::TerminalSession;

/// Time between pending-input flush checks when no terminal input is available.
const DEFAULT_EVENT_POLL_INTERVAL: Duration = Duration::from_millis(16);

/// Runs a root value in a managed terminal session.
#[derive(Debug)]
pub struct App<R> {
    /// Root view or runtime adapter rendered by the app loop.
    root: R,
    /// Component-facing runtime handle scoped to this runner.
    handle: AppHandle,
    /// Active standalone error screen scoped to this runner.
    error_screens: ErrorScreenRegistry,
    /// Destinations visited during this runner's lifetime.
    visited_links: VisitedLinkRegistry,
    /// Event polling timeout and, when enabled, idle redraw cadence.
    redraw_interval: Duration,
    /// Whether a timed-out event poll requests an idle redraw.
    redraw_on_timeout: bool,
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
    /// An [`App`] configured for event-driven redraws and default input polling.
    pub fn new(root: impl IntoView) -> Self {
        Self {
            root: root.into_view(),
            handle: AppHandle::new(),
            error_screens: ErrorScreenRegistry::new(),
            visited_links: VisitedLinkRegistry::new(),
            redraw_interval: DEFAULT_EVENT_POLL_INTERVAL,
            redraw_on_timeout: false,
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
    /// An [`App`] configured for event-driven redraws and default input polling.
    pub fn from_root(root: R) -> Self {
        Self {
            root,
            handle: AppHandle::new(),
            error_screens: ErrorScreenRegistry::new(),
            visited_links: VisitedLinkRegistry::new(),
            redraw_interval: DEFAULT_EVENT_POLL_INTERVAL,
            redraw_on_timeout: false,
        }
    }

    /// Enables periodic idle redraws at the requested interval.
    ///
    /// # Arguments
    ///
    /// * `redraw_interval` — Non-zero event polling timeout and idle redraw
    ///   cadence. Input and async redraw requests can wake the runner before
    ///   this timeout.
    ///
    /// # Returns
    ///
    /// An [`App`] configured to redraw periodically at the provided interval.
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
        self.redraw_on_timeout = true;
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
    fn handle_active_event(&mut self, event: crossterm::event::Event) -> Result<EventOutcome> {
        let visited_links = self.visited_links.clone();
        visited_links.with(|| {
            if let Some(mut screen) = self.error_screens.active() {
                View::handle_event(&mut screen, event).map(EventOutcome::recompute)
            } else {
                self.root
                    .__handle_event(event)
                    .map(|(control, reuse_layout)| {
                        if reuse_layout {
                            EventOutcome::reuse(control)
                        } else {
                            EventOutcome::recompute(control)
                        }
                    })
            }
        })
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

    /// Returns whether one completed event poll requires a redraw.
    ///
    /// Real events and flushed pending input always redraw. A timeout without
    /// either redraws only when periodic idle redraws were explicitly enabled.
    fn should_redraw_after_poll(&self, had_events: bool, flushed_input: bool) -> bool {
        had_events || flushed_input || self.redraw_on_timeout
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
        let mut next_layout = LayoutMode::Recompute;
        let mut last_viewport = None;
        let event_poll = next_events(self.redraw_interval);
        tokio::pin!(event_poll);

        'app: loop {
            if should_draw {
                let viewport = session.terminal.size()?;
                let layout = if next_layout == LayoutMode::Reuse && last_viewport == Some(viewport)
                {
                    LayoutMode::Reuse
                } else {
                    LayoutMode::Recompute
                };
                let visited_links = self.visited_links.clone();
                visited_links.with(|| {
                    draw_root(
                        &mut self.root,
                        &mut session.terminal,
                        &session.terminal_images,
                        &self.handle,
                        &self.error_screens,
                        layout,
                    )
                })?;
                last_viewport = Some(viewport);
                should_draw = false;
                next_layout = LayoutMode::Recompute;
            }

            tokio::select! {
                events = &mut event_poll => {
                    let events = events?;
                    let had_events = !events.is_empty();
                    let flushed_input = if had_events {
                        let mut batch_layout = LayoutMode::Reuse;
                        for event in events {
                            let outcome = self.handle_active_event(event)?;
                            if outcome.control == AppControl::Exit {
                                break 'app;
                            }
                            batch_layout = batch_layout.merge(outcome.layout);

                            if self.run_suspended_tasks(session)? {
                                batch_layout = LayoutMode::Recompute;
                                break;
                            }
                        }

                        next_layout = batch_layout;
                        false
                    } else {
                        let flushed_input = self.flush_active_input();
                        if flushed_input == Some(AppControl::Exit) {
                            break;
                        }

                        flushed_input.is_some()
                    };

                    should_draw = self.should_redraw_after_poll(had_events, flushed_input);
                    if flushed_input || self.redraw_on_timeout {
                        next_layout = LayoutMode::Recompute;
                    }
                    event_poll.set(next_events(self.redraw_interval));
                }
                changed = redraw_requests.changed() => {
                    if changed.is_ok() {
                        should_draw = true;
                        next_layout = LayoutMode::Recompute;
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

    /// Verifies default redraws are event driven and explicit intervals remain periodic.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// default app: timeout, event, flushed input
    /// periodic app: timeout
    /// ```
    ///
    /// # Assertions
    ///
    /// - An idle timeout does not redraw a default app.
    /// - Real events and flushed pending input redraw a default app.
    /// - An explicit redraw interval retains periodic idle redraws.
    #[test]
    fn redraws_are_event_driven_unless_periodic_redraws_are_enabled() {
        let default_app = App::new(text("test"));
        assert!(!default_app.should_redraw_after_poll(false, false));
        assert!(default_app.should_redraw_after_poll(true, false));
        assert!(default_app.should_redraw_after_poll(false, true));

        let periodic_app = App::new(text("test")).with_redraw_interval(Duration::from_millis(50));
        assert!(periodic_app.should_redraw_after_poll(false, false));
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
            EventOutcome::recompute(AppControl::Exit),
        );
        assert_eq!(event_count.get(), 0);

        app.error_screens.dismiss();
        assert_eq!(
            app.handle_active_event(key(KeyCode::Enter))
                .expect("ordinary root event dispatch should succeed"),
            EventOutcome::recompute(AppControl::Continue),
        );
        assert_eq!(event_count.get(), 1);
    }
}
