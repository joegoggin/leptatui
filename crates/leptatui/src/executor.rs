//! Asynchronous task and executor helpers.

use std::future::Future;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use crate::app::request_redraw;

/// Initializes the Any Spawner Tokio executor used by Leptos effects.
pub(crate) fn init_tokio_executor() {
    let _ = any_spawner::Executor::init_tokio();
}

/// Spawns a thread-safe future with the current reactive owner.
///
/// Completion requests an immediate terminal redraw.
///
/// # Arguments
///
/// * `future` — Asynchronous task to run on the Tokio executor.
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    init_tokio_executor();
    leptos::task::spawn(async move {
        future.await;
        request_redraw();
    });
}

/// Spawns a thread-local future with the current reactive owner.
///
/// Completion requests an immediate terminal redraw. The managed [`App`]
/// runtime provides the Tokio local task set required by this function.
///
/// # Arguments
///
/// * `future` — Thread-local asynchronous task to run.
///
/// # Panics
///
/// Panics if called outside a managed [`App`] runtime or another Tokio local
/// task set.
///
/// [`App`]: crate::App
pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    init_tokio_executor();
    leptos::task::spawn_local(async move {
        future.await;
        request_redraw();
    });
}

/// Tracks the latest generation of spawned async work.
#[derive(Clone, Default)]
pub(crate) struct LatestTask {
    generation: Arc<AtomicU64>,
}

impl LatestTask {
    /// Advances to the next generation and returns its id.
    pub(crate) fn next(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Returns whether `generation` is still the latest task generation.
    pub(crate) fn is_current(&self, generation: u64) -> bool {
        self.generation.load(Ordering::Acquire) == generation
    }
}

#[cfg(test)]
/// Unit tests for asynchronous task helpers.
mod tests {
    use std::{cell::Cell, rc::Rc};

    use tokio::{task::yield_now, time::timeout};

    use super::*;

    /// Verifies local tasks run inside the managed local-task environment.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// LocalSet::run_until(spawn_local(set completed))
    /// ```
    ///
    /// # Assertions
    ///
    /// - The local future can capture non-send state.
    /// - The local future completes within the test timeout.
    #[tokio::test(flavor = "current_thread")]
    async fn local_task_runs_inside_local_set() {
        let completed = Rc::new(Cell::new(false));
        let completed_by_task = Rc::clone(&completed);

        tokio::task::LocalSet::new()
            .run_until(async move {
                spawn_local(async move {
                    completed_by_task.set(true);
                });

                timeout(std::time::Duration::from_secs(1), async {
                    while !completed.get() {
                        yield_now().await;
                    }
                })
                .await
                .expect("local task should complete");
            })
            .await;
    }
}
