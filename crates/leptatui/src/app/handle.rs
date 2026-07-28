//! Component-facing application runtime handle.
//!
//! This module exposes a scoped handle that lets managed components request
//! terminal suspension while synchronous external work owns the terminal.

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, ThreadId},
};

use crate::context;

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
}

/// Component-facing handle for the active [`App`](super::App) runner.
///
/// The handle is provided through context before the root component is
/// materialized. Clones refer to the same runner-local terminal-task queue.
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
    /// An [`AppHandle`] with no queued terminal tasks.
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(AppHandleState {
                id: NEXT_APP_HANDLE_ID.fetch_add(1, Ordering::Relaxed),
                owner_thread: thread::current().id(),
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
    /// Formats queue state without inspecting task closures.
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
        formatter
            .debug_struct("AppHandle")
            .field("suspended_tasks", &suspended_tasks)
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
    use std::{cell::RefCell, rc::Rc};

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
}
