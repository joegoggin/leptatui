//! Component-facing application runtime handle.
//!
//! This module exposes a scoped handle that lets managed components request
//! terminal suspension and preserve an application error for return after the
//! user exits an error screen.

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    error::Error as StdError,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, ThreadId},
};

use crate::context;

use super::Error;

/// Synchronous task executed while the managed terminal is restored.
type SuspendedTask = Box<dyn FnOnce()>;

/// Next process-local identifier assigned to an application handle.
static NEXT_APP_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// Same-thread task queues keyed by their runner handle.
    static SUSPENDED_TASKS: RefCell<HashMap<u64, VecDeque<SuspendedTask>>> =
        RefCell::new(HashMap::new());
}

/// Thread-safe metadata shared by clones of one application handle.
struct AppHandleState {
    /// Process-local identifier used to find the same-thread task queue.
    id: u64,
    /// Thread that owns terminal work for this handle.
    owner_thread: ThreadId,
    /// First application error to return after a requested exit.
    exit_error: Mutex<Option<Error>>,
}

/// Component-facing handle for the active [`App`](super::App) runner.
///
/// The handle is provided through context before the root component is
/// materialized. Clones refer to the same runner-local task queue and deferred
/// error.
#[derive(Clone)]
pub struct AppHandle {
    /// Shared metadata for this runner.
    state: Arc<AppHandleState>,
}

impl AppHandle {
    /// Creates an empty handle for one app runner.
    ///
    /// # Returns
    ///
    /// An [`AppHandle`] with no queued tasks or deferred error.
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(AppHandleState {
                id: NEXT_APP_HANDLE_ID.fetch_add(1, Ordering::Relaxed),
                owner_thread: thread::current().id(),
                exit_error: Mutex::new(None),
            }),
        }
    }

    /// Queues work to run while Leptatui temporarily releases the terminal.
    ///
    /// Tasks run synchronously on the runner thread after the active event
    /// handler returns. The runner restores normal terminal modes, executes
    /// every queued task in FIFO order, re-enters the managed terminal, and
    /// redraws the existing component tree.
    ///
    /// # Arguments
    ///
    /// * `task` — Synchronous work that needs normal terminal ownership.
    ///
    /// # Panics
    ///
    /// Panics if called from a thread other than the one that created the
    /// handle.
    pub fn suspend_terminal(&self, task: impl FnOnce() + 'static) {
        self.assert_owner_thread();
        SUSPENDED_TASKS.with(|queues| {
            queues
                .borrow_mut()
                .entry(self.state.id)
                .or_default()
                .push_back(Box::new(task));
        });
    }

    /// Records an application error to return after the component tree exits.
    ///
    /// The first recorded error wins. Recording an error does not exit the app,
    /// which allows a component to render a diagnostic screen before returning
    /// the original failure.
    ///
    /// # Arguments
    ///
    /// * `error` — Application failure to return from [`App::run`](super::App::run).
    pub fn set_exit_error<E>(&self, error: E)
    where
        E: StdError + Send + Sync + 'static,
    {
        let mut exit_error = self
            .state
            .exit_error
            .lock()
            .expect("app handle exit error should not be poisoned");
        if exit_error.is_none() {
            *exit_error = Some(Error::Application(Box::new(error)));
        }
    }

    /// Removes all currently queued terminal-suspension tasks.
    ///
    /// # Returns
    ///
    /// A [`VecDeque`] containing tasks in submission order.
    ///
    /// # Panics
    ///
    /// Panics if called from a thread other than the one that created the
    /// handle.
    pub(crate) fn take_suspended_tasks(&self) -> VecDeque<SuspendedTask> {
        self.assert_owner_thread();
        SUSPENDED_TASKS.with(|queues| {
            queues
                .borrow_mut()
                .remove(&self.state.id)
                .unwrap_or_default()
        })
    }

    /// Removes the deferred application error, if one was recorded.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the first recorded application error.
    pub(crate) fn take_exit_error(&self) -> Option<Error> {
        self.state
            .exit_error
            .lock()
            .expect("app handle exit error should not be poisoned")
            .take()
    }

    /// Verifies a same-thread operation runs on the handle's owner thread.
    ///
    /// # Panics
    ///
    /// Panics if the current thread does not own this handle.
    fn assert_owner_thread(&self) {
        assert_eq!(
            thread::current().id(),
            self.state.owner_thread,
            "AppHandle terminal tasks must be queued and executed on the runner thread"
        );
    }
}

impl fmt::Debug for AppHandle {
    /// Formats queue and error presence without inspecting task closures.
    ///
    /// # Arguments
    ///
    /// * `formatter` — Debug formatter receiving the handle summary.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating whether formatting succeeded.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suspended_tasks = if thread::current().id() == self.state.owner_thread {
            SUSPENDED_TASKS
                .with(|queues| queues.borrow().get(&self.state.id).map_or(0, VecDeque::len))
        } else {
            0
        };
        let has_exit_error = self
            .state
            .exit_error
            .lock()
            .expect("app handle exit error should not be poisoned")
            .is_some();

        formatter
            .debug_struct("AppHandle")
            .field("suspended_tasks", &suspended_tasks)
            .field("has_exit_error", &has_exit_error)
            .finish()
    }
}

/// Returns the runtime handle for the component managed by the active app.
///
/// # Returns
///
/// An [`AppHandle`] connected to the nearest managed app runner.
///
/// # Panics
///
/// Panics if no [`App`](super::App) runtime handle exists in context.
#[track_caller]
pub fn use_app_handle() -> AppHandle {
    context::expect_context::<AppHandle>()
}

#[cfg(test)]
/// Unit tests for runner-local application handles.
mod tests {
    use std::{cell::RefCell, io, rc::Rc};

    use super::*;

    /// Verifies suspended tasks retain submission order.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// suspend_terminal(push 1)
    /// suspend_terminal(push 2)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Two tasks are queued.
    /// - Executing the drained tasks records `1, 2`.
    #[test]
    fn suspended_tasks_are_drained_in_fifo_order() {
        let handle = AppHandle::new();
        let values = Rc::new(RefCell::new(Vec::new()));

        for value in [1, 2] {
            let values = values.clone();
            handle.suspend_terminal(move || values.borrow_mut().push(value));
        }

        let tasks = handle.take_suspended_tasks();
        assert_eq!(tasks.len(), 2);
        for task in tasks {
            task();
        }
        assert_eq!(*values.borrow(), vec![1, 2]);
    }

    /// Verifies the first deferred application error wins.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// set_exit_error("first")
    /// set_exit_error("second")
    /// ```
    ///
    /// # Assertions
    ///
    /// - Taking the deferred error returns an application error.
    /// - The retained source is the first submitted failure.
    /// - A second take returns no error.
    #[test]
    fn first_exit_error_is_retained_until_taken() {
        let handle = AppHandle::new();
        handle.set_exit_error(io::Error::other("first"));
        handle.set_exit_error(io::Error::other("second"));

        let error = handle
            .take_exit_error()
            .expect("the first application error should be retained");
        assert_eq!(
            StdError::source(&error).map(ToString::to_string).as_deref(),
            Some("first")
        );
        assert!(handle.take_exit_error().is_none());
    }
}
